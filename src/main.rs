use chrono::{DateTime, FixedOffset, NaiveTime, Utc};

/// 与えられたNaiveTimeと現在の時刻を比較します
/// * `end_time` - 比較対象の時間
fn comp_end_time(end_time: NaiveTime) -> bool {
    // let end_time: NaiveTime = NaiveTime::from_hms_opt(13, 5, 0).unwrap();
    let now_utc: DateTime<Utc> = Utc::now();
    let now: DateTime<FixedOffset> = now_utc.with_timezone(&FixedOffset::east_opt(9*3600).unwrap());
    let now_naive: NaiveTime = now.time();
    now_naive > end_time
}

fn main() {
    print!("{}", comp_end_time(NaiveTime::from_hms_opt(13, 5, 0).unwrap()));
}