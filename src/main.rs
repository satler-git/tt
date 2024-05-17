//! ## configについて
//! * `end_time`: [h, m, s]
//! 24h表記
//! * `api_key`
//! gasのデプロイID

use chrono::{DateTime, Duration,FixedOffset, NaiveTime, Utc};
use clap::Parser;
use directories::ProjectDirs;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rand::seq::SliceRandom;
use rusqlite::{params, Connection, Result};
use serde_derive::{Deserialize, Serialize};
use std::process::{Command, Stdio};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// duration to play
    #[arg(short, long, conflicts_with = "end_time",help = "Specify the time to play in the form of \"1h30m5s\"")]
    duration: Option<String>,

    #[arg(short, long, conflicts_with = "duration",help = "\"%H:%M\", 24h")]
    end_time: Option<String>,

    #[arg(last = true, help = "Options for mpv")]
    mpv_args: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
struct MyConfig {
    api_key: String,
    end_time: [u32; 3],
}

/// `MyConfig` implements `Default`
impl ::std::default::Default for MyConfig {
    fn default() -> Self {
        Self {
            api_key: "".into(),
            end_time: [13, 5, 0],
        }
    }
}

/// `1h30m5s`のような時間をパースする関数
fn parse_duration_time(time_str: &str) -> Duration {
    let mut parts = time_str.split(|c| c == 'h' || c == 'm' || c == 's');
    let hours = parts.next().unwrap_or("0").parse::<u32>().unwrap();
    let minutes = parts.next().unwrap_or("0").parse::<u32>().unwrap();
    let seconds = parts.next().unwrap_or("0").parse::<u32>().unwrap();
    Duration::seconds((seconds + minutes * 60u32 + hours * 3600u32) as i64)
}

fn parse_end_time_from_str(time_str: &str) -> NaiveTime {
    NaiveTime::parse_from_str(time_str, "%H:%M").expect("Can't parse end time from str")
}

fn parse_time_options(args: Args) -> Result<[u32; 3]> {
    let mut end_time = match args.duration {
        Some(i) => {
            Utc::now().time() + parse_duration_time(&i)
        },
        _ => {return Err(()},
    };
    end_time = match args.end_time {
        Some(i) => {
            parse_end_time_from_str(&i)
        }
        _ => {return Err(()}
    };
    Ok([end_time.hour(), end_time.minute(), end_time.second()])
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

/// `comp_end_time()`のラップ
/// * `cfg` Myconfig
fn comp_time(cfg: &MyConfig) -> bool {
    comp_end_time(
        NaiveTime::from_hms_opt(cfg.end_time[0], cfg.end_time[1], cfg.end_time[2]).unwrap(),
    )
}

/// 検索結果用の構造体
#[derive(Deserialize, Debug)]
struct YoutubeSearchResult {
    items: Vec<YoutubeSearchItem>,
}

#[derive(Deserialize, Debug)]
struct YoutubeSearchItem {
    id: YoutubeSearchId,
}

#[allow(non_snake_case)]
#[derive(Deserialize, Debug)]
struct YoutubeSearchId {
    videoId: String,
}

/// 与えられた単語のリストからvideo_idを取得します
/// * `search_word_list` - 検索する単語のリスト
/// Youtubeのリンクとして返します
async fn search_youtube(search_word_list: [&String; 2]) -> String {
    let [name, artist] = search_word_list;
    let request_url = format!(
        "https://yt.lemnoslife.com/search?part=id&q={name}+{artist}&type=video",
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
    format!(
        "https://www.youtube.com/watch?v={video_id}",
        video_id = result.items[0].id.videoId.clone()
    )
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
async fn play_music(search_word_list: [&String; 2], mpv_args: &Vec<String>) {
    println!("Playing {} {}", search_word_list[0], search_word_list[1]);

    let video_id = search_youtube(search_word_list).await;

    match Command::new("mpv")
        .args( mpv_args + Vec::from(["-fs", /* "--volume=50", */ &video_id]))
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
    async fn play(&self, mpv_args: &Vec<String>) {
        play_music([&self.song_name, &self.artist_name], mpv_args).await;
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

    for song in backend_result.contents {
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

/// SQLiteから次の流すべきリクエストを判断し`play_song()`で再生
/// * `conn` SQLiteのコネクション
async fn play_next(conn: &Connection, mpv_args: &Vec<String>) {
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

    next.play(mpv_args).await;
    next.set_as_played(&conn);
}

#[tokio::main]
async fn main() -> Result<(), confy::ConfyError> {
    let mut cfg: MyConfig = confy::load("tt", "tt")?;
    let args = Args::parse();
    if let Ok(i)  = parse_time_options(&args) { 
        cfg.end_time = i;
    }
    let conn = init_sqlite().unwrap();
    sync_backend(&cfg, &conn).await.unwrap();

    let mpv_args = match args.mpv_args {
        Some(i) => i,
        _ => Vec::new()
    };

    while comp_time(&cfg) {
        play_next(&conn, &mpv_args).await;
        println!("Comp to time: {}", comp_time(&cfg));
    }

    Ok(())
}
