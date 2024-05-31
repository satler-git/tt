//! ## configについて
//! * `end_time`: [h, m, s]
//! 24h表記
//! * `api_key`
//! gasのデプロイID

use chrono::{DateTime, FixedOffset, NaiveTime, Timelike, Utc};
use clap::Parser;
use directories::ProjectDirs;
use indicatif::ProgressIterator;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rand::seq::SliceRandom;
use rusqlite::{params, Connection, Result};
use serde_derive::{Deserialize, Serialize};
use std::{
    process::{Command, Stdio},
    u32,
};

#[derive(Serialize, Deserialize)]
struct MyConfig {
    api_key: String,
    end_time: [u32; 3],
}

/// `MyConfig` implements `Default`
impl Default for MyConfig {
    fn default() -> Self {
        Self {
            api_key: "".into(),
            end_time: [13, 5, 0],
        }
    }
}

/// 与えられたNaiveTimeと現在の時刻を比較します
/// * `end_time` - 比較対象の時間
fn comp_end_time(end_time: NaiveTime) -> bool {
    // let end_time: NaiveTime = NaiveTime::from_hms_opt(13, 5, 0).unwrap();
    let now_utc: DateTime<Utc> = Utc::now();
    let now: DateTime<FixedOffset> =
        now_utc.with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap());
    let now_naive: NaiveTime = now.time();
    now_naive < end_time
}

/// 検索結果用の構造体
#[derive(Deserialize, Debug)]
struct YoutubeSearchResult {
    items: Vec<YoutubeSearchItem>,
}

#[derive(Deserialize, Debug)]
struct YoutubeSearchItem {
    id: YoutubeSearchId,
    /* snippet: YoutubeSearchSnippet, */
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct YoutubeSearchId {
    videoId: String,
}

/*
#[derive(Deserialize, Debug)]
struct YoutubeSearchSnippet {
    duration: i64,
}
*/

/// 与えられた単語のリストからvideo_idを取得します
/// * `search_word_list` - 検索する単語のリスト
/// Youtubeのリンクとして返します
async fn search_youtube(search_word_list: [&String; 2]) -> String {
    let [name, artist] = search_word_list;
    let request_url = format!(
        "https://yt.lemnoslife.com/search?part=id,snippet&q={name}+{artist}&type=video",
        name = utf8_percent_encode(&name, NON_ALPHANUMERIC).to_string(),
        artist = utf8_percent_encode(&artist, NON_ALPHANUMERIC).to_string()
    );

    let body = reqwest::get(&request_url)
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let result: YoutubeSearchResult = serde_json::from_str(&body).unwrap();
    for i in &result.items {
        // if i.snippet.duration >= 900 {
            // 60s * 15m
            return format!(
                "https://www.youtube.com/watch?v={video_id}",
                video_id = result.items[0].id.videoId.clone()
            );
        /*
        } else {
            println!("The video is over 15 minutes long.")
        }
        */
    }
    println!("Couldn't find video with search.");
    return "https://github.com/satler-git/tt/releases/download/v2.0.1/error.wav".to_string();
}

/// SQLiteから取得し、再生するためのstruct
#[derive(Deserialize)]
struct Request {
    id: i32,
    song_name: String,
    artist_name: String,
}

/// 与えられた単語のリストからvideo_idを取得してmpvで再生します
/// * `search_word_list` - 検索する単語のリスト
async fn play_music(search_word_list: [&String; 2], mpv_arsg: &Option<Vec<String>>) {
    println!("Playing {} {}", search_word_list[0], search_word_list[1]);

    let video_id = search_youtube(search_word_list).await;
    let mut mpv_options: Vec<String> = vec![];
    if let Some(mo) = mpv_arsg {
        mpv_options = mo.clone();
    }
    // デフォルトのオプションを追加
    mpv_options.push("-fs".into());
    mpv_options.push(video_id.into());
    match Command::new("mpv")
        .args(mpv_options)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
    {
        Err(why) => panic!("Error on executing mpv: {}", why),
        Ok(process) => process,
    };
}

impl Request {
    /// `play_music()`で再生します
    async fn play(&self, mpv_arsg: &Option<Vec<String>>) {
        play_music([&self.song_name, &self.artist_name], mpv_arsg).await;
    }

    fn set_as_played(&self, conn: &Connection) {
        conn.execute(
            "UPDATE requests SET played = 1 WHERE id = ?1",
            params![self.id],
        )
        .unwrap();
    }
}

/// SQLiteをセットアップしコネクションを返す
fn init_sqlite() -> Result<Connection, rusqlite::Error> {
    println!("Initialing SQLite");

    let binding = ProjectDirs::from("com", "", "tt").unwrap();
    let project_dir = binding.config_dir();
    let db_path = project_dir.join("tt.sqlite3");

    let conn = Connection::open(&db_path)?;

    let is_autocommit = conn.is_autocommit();
    println!("    Is auto-commit mode: {}", is_autocommit);

    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS requests(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL,
            song_name TEXT NOT NULL,
            artist_name TEXT NOT NULL,
            played INTEGER NOT NULL,
            uuid TEXT NOT NULL,
            arrange INTEGER NOT NULL,
            UNIQUE(uuid)
        );
    ",
        params![],
    )?;

    Ok(conn)
}

/// gasバックエンド用のstruct
#[derive(Deserialize, Debug)]
struct BackendResult {
    contents: Vec<BackendSong>,
}

/// BackendResultの子struct
#[derive(Deserialize, Debug)]
struct BackendSong {
    mail: String,
    song_name: String,
    artist_name: String,
    uuid: String,
}

/// Backendと同期するための関数
/// * `cfg` アプリの設定
/// * `conn` SQLiteへのコネクション
async fn sync_backend(cfg: &MyConfig, conn: &Connection) -> Result<(), rusqlite::Error> {
    println!("Syncing SQLite");

    let request_url = format!(
        "https://script.google.com/macros/s/{api_key}/exec",
        api_key = cfg.api_key
    );

    let body = reqwest::get(&request_url)
        .await
        .unwrap()
        .text()
        .await
        .unwrap();

    let backend_result: BackendResult = serde_json::from_str(&body).unwrap();

    let mut stmt = conn
        .prepare("select id from requests where email = ?1")
        .unwrap();

    for song in backend_result.contents.into_iter().progress() {
        let order = stmt
            .query([&song.mail])
            .unwrap()
            .mapped(|_row| Ok(0))
            .count()
            + 1;
        conn.execute("INSERT OR IGNORE INTO requests(email, song_name, artist_name, played, uuid, arrange) VALUES(?1, ?2, ?3, 0, ?4, ?5)", params![&song.mail, &song.song_name, &song.artist_name, &song.uuid, &order])?;
    }
    Ok(())
}

/// `comp_end_time()`のラップ
/// * `cfg` Myconfig
fn comp_time(cfg: &MyConfig) -> bool {
    comp_end_time(
        NaiveTime::from_hms_opt(cfg.end_time[0], cfg.end_time[1], cfg.end_time[2]).unwrap(),
    )
}

/// SQLiteから次の流すべきリクエストを判断し`play_song()`で再生
/// * `conn` SQLiteのコネクション
/// * `mpv_arsg` MPVへのオプションのオプション
async fn play_next(conn: &Connection, mpv_arsg: &Option<Vec<String>>) {
    // playedがfalseかつ、arrangeが最小(!unique)
    let mut stmt = conn
        .prepare("select id, song_name, artist_name from requests where played = 0 and arrange = (select MIN(arrange) from requests where played = 0)")
        .unwrap();
    let request_iter = stmt
        .query_map([], |row| {
            Ok(Request {
                id: row.get(0).unwrap(),
                song_name: row.get(1).unwrap(),
                artist_name: row.get(2).unwrap(),
            })
        })
        .unwrap();

    let mut requests = Vec::new();

    for request in request_iter {
        requests.push(request.unwrap());
    }

    if requests.len() == 0 {
        panic!("Couldn't find next to play");
    }

    let next = requests.choose(&mut rand::thread_rng()).unwrap();

    next.play(mpv_arsg).await;
    next.set_as_played(&conn);
}

/// ClI引数用のストラクト
#[derive(Parser, Debug)]
#[command(version)]
#[command(about = "A CLI tool for playing music automatically.", long_about = None)]
struct Args {
    // "12:30"のように24hで指定する
    #[arg(
        short,
        long,
        help = "Specify the end time in 24-hour notation, separated by \":\", for example, \"12:30\" minutes. \nCan't be specified at the same time as the duration."
    )]
    #[arg(conflicts_with = "duration")]
    end_time: Option<String>,
    // "1h2m3s"的な感じで指定する
    #[arg(
        short,
        long,
        help = "Specify by integer, separated by h, m, and s, as in \"1h2m3s\". Each can be omitted."
    )]
    duration: Option<String>,
    #[arg(last = true, help = "Arguments passed directly to mpv.")]
    mpv_arsg: Option<Vec<String>>,
}

/// `12:30`の様に与えられたのを`[12, 30, 0]`の様にパースする
/// `26:40`の様に与えられた場合はパニック
fn parse_end_time(end_time: String) -> [u32; 3] {
    let str_words: Vec<&str> = end_time.split(":").collect();

    let u32_words = str_words
        .iter()
        .map(|s| s.parse::<u32>().unwrap())
        .collect::<Vec<u32>>();

    // 正しい数じゃないか確認
    if (u32_words[0] >= 24) | (u32_words[1] >= 60) | (str_words.len() != 2) {
        panic!("The end_time option was invalid.");
    }

    [u32_words[0], u32_words[1], 0]
}

/// `[12, 70, 0]`を`[13, 10, 0]`にmodする関数
fn mod_time(time: [u32; 3]) -> [u32; 3] {
    [
        (time[0] + time[1] / 60 + time[2] / 3600) % 24,
        (time[1] + time[2] / 60) % 60,
        time[2] % 60,
    ]
}

/// `1h2m30s`の様な入力を`[1, 2, 30]`の様に返す
/// それぞれ省略可能で例えば`1h30m`が`[1, 30, 0]`
/// 24時間以上はpanic
fn parse_duration_diff(duration: String) -> [u32; 3] {
    let mut h_point = 0;
    let mut m_point = 0;
    let mut h = 0;
    let mut m = 0;
    let mut s = 0;
    if duration.contains("h") {
        h_point = duration.find("h").unwrap();
        h = String::from(&duration[0..h_point])
            .parse()
            .expect("The duration option was invalid.");
        h_point += 1;
    }
    if duration.contains("m") {
        m_point = duration.find("m").unwrap();
        m = String::from(&duration[h_point..m_point])
            .parse()
            .expect("The duration option was invalid.");
        m_point += 1;
    } else if duration.contains("h") {
        m_point = h_point.clone();
    }
    if duration.contains("s") {
        s = String::from(&duration[m_point..duration.find("s").unwrap()])
            .parse()
            .expect("The duration option was invalid.");
    }
    mod_time([h, m, s])
}

/// `parse_duration_diff()`を使用して現在時刻と足しあわせる関数
fn parse_duration(duration: String) -> [u32; 3] {
    let diff = parse_duration_diff(duration);
    let now = Utc::now()
        .with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap())
        .time();
    mod_time([
        diff[0] + now.hour(),
        diff[1] + now.minute(),
        diff[2] + now.second(),
    ])
}

#[tokio::main]
async fn main() -> Result<(), confy::ConfyError> {
    let mut cfg: MyConfig = confy::load("tt", "tt")?;
    let args = Args::parse();
    // argsをcfgに反映
    if let Some(end_time) = args.end_time {
        cfg.end_time = parse_end_time(end_time);
    } else if let Some(duration) = args.duration {
        cfg.end_time = parse_duration(duration);
    }
    let conn = init_sqlite().unwrap();
    sync_backend(&cfg, &conn).await.unwrap();
    println!("Comp to time: {}", comp_time(&cfg));
    while comp_time(&cfg) {
        play_next(&conn, &args.mpv_arsg).await;
        println!("Comp to time: {}", comp_time(&cfg));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 正常な入力をパース出来るか
    #[test]
    fn check_parse_end_time() {
        assert_eq!([12, 30, 0], parse_end_time("12:30".into()));
        assert_eq!([6, 6, 0], parse_end_time("6:6".into()));
        assert_eq!([13, 5, 0], parse_end_time("13:05".into()));
    }

    #[test]
    #[should_panic]
    fn check_parse_end_time_invaild_h() {
        parse_end_time("100:0".into());
    }
    #[test]
    #[should_panic]
    fn check_parse_end_time_invaild_m() {
        parse_end_time("0:100".into());
    }
    #[test]
    #[should_panic]
    fn check_parse_end_time_invaild_length() {
        parse_end_time("0:0:0".into());
    }
    #[test]
    #[should_panic]
    fn check_parse_end_time_invaild_h_minus() {
        parse_end_time("-1:0".into());
    }

    #[test]
    fn check_mod_time() {
        assert_eq!([2, 0, 0], mod_time([1, 60, 0]));
        assert_eq!([1, 0, 0], mod_time([0, 0, 3600]));
    }
    #[test]
    fn check_parse_duration() {
        assert_eq!([1, 2, 3], parse_duration_diff("1h2m3s".into()));
        assert_eq!([1, 2, 0], parse_duration_diff("1h2m".into()));
        assert_eq!([0, 2, 3], parse_duration_diff("2m3s".into()));
        assert_eq!([2, 2, 0], parse_duration_diff("1h62m".into()));
        assert_eq!([1, 0, 3], parse_duration_diff("1h3s".into()));
    }
    /// テスト用のインメモリSQLiteのコネクションを作成
    fn create_sqlite_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "
                CREATE TABLE requests(
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    email TEXT NOT NULL,
                    song_name TEXT NOT NULL,
                    artist_name TEXT NOT NULL,
                    played INTEGER NOT NULL,
                    uuid TEXT NOT NULL,
                    arrange INTEGER NOT NULL,
                    UNIQUE(uuid)
                );
            ",
            params![],
        )
        .unwrap();
        conn
    }
}
