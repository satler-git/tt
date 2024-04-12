//! ## configについて
//! * `end_time`: [h, m, s]
//! 24h表記
//! * `api_key`
//! gasのデプロイID

use chrono::{DateTime, FixedOffset, NaiveTime, Utc};
use directories::ProjectDirs;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rusqlite::{params, Connection, Result};
use serde_derive::{Deserialize, Serialize};
use std::process::{Command, Stdio};

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

/// 与えられたNaiveTimeと現在の時刻を比較します
/// * `end_time` - 比較対象の時間
fn comp_end_time(end_time: NaiveTime) -> bool {
    // let end_time: NaiveTime = NaiveTime::from_hms_opt(13, 5, 0).unwrap();
    let now_utc: DateTime<Utc> = Utc::now();
    let now: DateTime<FixedOffset> =
        now_utc.with_timezone(&FixedOffset::east_opt(9 * 3600).unwrap());
    let now_naive: NaiveTime = now.time();
    now_naive > end_time
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
async fn search_youtube(search_word_list: [String; 2]) -> String {
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

/// 与えられた単語のリストからvideo_idを取得してmpvで再生します
/// * `search_word_list` - 検索する単語のリスト
async fn play_music(search_word_list: [String; 2]) {
    let video_id = search_youtube(search_word_list).await;

    match Command::new("mpv")
        .args(["-fs", "--volume=50", &video_id])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .output()
    {
        Err(why) => panic!("Error on executing mpv: {}", why),
        Ok(process) => process,
    };
}

/// SQLiteをセットアップしコネクションを返す
#[allow(dead_code)]
fn init_sqlite() -> Result<Connection, rusqlite::Error> {
    let binding = ProjectDirs::from("com", "", "tt").unwrap();
    let project_dir = binding.config_dir();
    let db_path = project_dir.join("tt.sqlite3");

    let conn = Connection::open(&db_path)?;

    let is_autocommit = conn.is_autocommit();
    println!("Is auto-commit mode: {}", is_autocommit);

    conn.execute(
        "
        CREATE TABLE IF NOT EXISTS requests(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            email TEXT NOT NULL,
            song_name TEXT NOT NULL,
            artist_name TEXT NOT NULL,
            played INTEGER NOT NULL,
            uuid TEXT NOT NULL,
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

    for song in backend_result.contents {
        conn.execute("INSERT OR IGNORE INTO requests(email, song_name, artist_name, played, uuid) VALUES(?1, ?2, ?3, 0, ?4)", params![song.mail, song.song_name, song.artist_name, song.uuid])?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), confy::ConfyError> {
    let cfg: MyConfig = confy::load("tt", "tt")?;
    let conn = init_sqlite().unwrap();
    sync_backend(&cfg, &conn).await.unwrap();
    // println!("{}", comp_end_time(NaiveTime::from_hms_opt(cfg.end_time[0], cfg.end_time[1], cfg.end_time[2]).unwrap()));
    // play_music(["再生".to_string(), "ナナツカゼ".to_string()]).await;
    Ok(())
}