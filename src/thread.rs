/*
Copyright (C) 2012, 2021 Kunal Mehta <legoktm@debian.org>

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */
use crate::mask::prefix_number;
use anyhow::Result;
use lazy_static::lazy_static;
use mwbot::parsoid::*;
use regex::Regex;
use time::{Date, Duration, Month, PrimitiveDateTime, Time};

#[derive(Clone, Debug)]
pub(crate) struct Thread {
    pub(crate) topic: String,
    pub(crate) replies: u32,
    pub(crate) link: String,
    pub(crate) first: PrimitiveDateTime,
    pub(crate) last: PrimitiveDateTime,
}

impl Thread {
    pub(crate) fn first(&self) -> String {
        mw_time_format(&self.first)
    }

    pub(crate) fn first_epoch(&self) -> i64 {
        self.first.assume_utc().unix_timestamp()
    }

    pub(crate) fn last(&self) -> String {
        mw_time_format(&self.last)
    }

    pub(crate) fn last_epoch(&self) -> i64 {
        self.last.assume_utc().unix_timestamp()
    }

    pub(crate) fn duration(&self) -> String {
        let dur = Duration::seconds(self.duration_secs());
        let days = dur.whole_days();
        let mut text = vec![];
        match days {
            1 => text.push("1 day, ".to_string()),
            2.. => text.push(format!("{} days, ", days)),
            _ => {}
        }
        let hours = dur.whole_hours() - (days * 24);
        let minutes = dur.whole_minutes() - (dur.whole_hours() * 60);
        let seconds = dur.whole_seconds() - (dur.whole_minutes() * 60);
        text.push(format!(
            "{}:{}:{}",
            prefix_number(hours as u32, 1),
            prefix_number(minutes as u32, 1),
            prefix_number(seconds as u32, 1)
        ));
        text.join("")
    }

    pub(crate) fn duration_secs(&self) -> i64 {
        self.last_epoch() - self.first_epoch()
    }
}

fn mw_time_format(dt: &PrimitiveDateTime) -> String {
    format!(
        "{}:{}, {} {} {}",
        prefix_number(dt.hour() as u32, 1),
        prefix_number(dt.minute() as u32, 1),
        dt.day(),
        dt.month(),
        dt.year()
    )
}

pub(crate) fn get_month(input: &str) -> Month {
    match input {
        "January" => Month::January,
        "February" => Month::February,
        "March" => Month::March,
        "April" => Month::April,
        "May" => Month::May,
        "June" => Month::June,
        "July" => Month::July,
        "August" => Month::August,
        "September" => Month::September,
        "October" => Month::October,
        "November" => Month::November,
        "December" => Month::December,
        // Unreachable because the regex should've only matched real months
        what => unreachable!("Invalid month {}", what),
    }
}

pub(crate) fn extract_threads(code: &Wikicode) -> Result<Vec<Thread>> {
    lazy_static! {
        static ref RE: Regex = {
            let raw =
                r"(\d{2}):(\d{2}|\d{2}:\d{2}:\d{2}), (\d{1,2}) $month (\d{4})"
                    .replace("$month", crate::MONTH_REGEX);
            Regex::new(&raw).unwrap()
        };
    }
    let mut threads = vec![];
    for section in code.iter_sections() {
        if let Some(heading) = section.heading() {
            if heading.get_level() != 2 {
                continue;
            }
            // TODO: upstream this to parsoid-rs
            let link = heading
                .as_element()
                .unwrap()
                .attributes
                .borrow()
                .get("id")
                .unwrap()
                .to_string();
            let page = code.get_title().unwrap();
            let link = format!("[[{}#{}]]", page, link.replace('_', " "));
            let text = section.text_contents();
            let mut timestamps = vec![];
            for capture in RE.captures_iter(&text) {
                // dbg!(&capture);

                let date = Date::from_calendar_date(
                    // unwrap: safe because the regex matched \d
                    capture[5].parse().unwrap(),
                    get_month(&capture[4]),
                    // unwrap: safe because the regex matched \d
                    capture[3].parse().unwrap(),
                );
                let date = match date {
                    Ok(date) => date,
                    Err(err) => {
                        println!("[[{}]]: invalid date, {:?}", &page, err);
                        // Skip this timestamp
                        continue;
                    }
                };
                let time = Time::from_hms(
                    // unwrap: safe because the regex matched \d
                    capture[1].parse().unwrap(),
                    // unwrap: safe because the regex matched \d
                    capture[2].parse().unwrap(),
                    0,
                );
                let time = match time {
                    Ok(time) => time,
                    Err(err) => {
                        println!("[[{}]]: invalid time, {:?}", &page, err);
                        // Skip this timestamp
                        continue;
                    }
                };
                let dt = PrimitiveDateTime::new(date, time);
                timestamps.push(dt);
            }
            // Just skip
            if timestamps.is_empty() {
                continue;
            }
            timestamps.sort();
            // dbg!(&timestamps);
            threads.push(Thread {
                topic: heading.text_contents(),
                replies: timestamps.len() as u32,
                link,
                first: *timestamps.first().unwrap(),
                last: *timestamps.last().unwrap(),
            })
        }
    }
    // dbg!(&threads);
    Ok(threads)
}
