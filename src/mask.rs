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
use crate::thread::{extract_threads, get_month, Thread};
use anyhow::{anyhow, Result};
use mwbot::{Bot, Error};
use regex::Regex;
use std::fmt;
use time::Month;

#[derive(Debug)]
pub(crate) enum Mask {
    // [[Talk:Foo/Archive <#>]]
    Numerical { mask: String, leading_zeros: u32 },
    // [[Talk:Foo bar]]
    SinglePage { title: String },
    Monthly { mask: String, first_archive: String },
    Yearly { mask: String, first_archive: String },
}

impl Mask {
    pub(crate) async fn expand(&self, bot: &Bot) -> Result<Vec<Thread>> {
        match self {
            Mask::Numerical {
                mask,
                leading_zeros,
            } => expand_numerical_mask(bot, mask, *leading_zeros).await,
            Mask::SinglePage { title } => expand_single_mask(bot, title).await,
            Mask::Monthly {
                mask,
                first_archive,
            } => expand_monthly_mask(bot, mask, first_archive).await,
            Mask::Yearly {
                mask,
                first_archive,
            } => expand_yearly_mask(bot, mask, first_archive).await,
        }
    }
}

impl fmt::Display for Mask {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Mask::Numerical { mask, .. } => mask,
            Mask::SinglePage { title } => title,
            Mask::Monthly { mask, .. } => mask,
            Mask::Yearly { mask, .. } => mask,
        };
        write!(f, "{}", name)
    }
}

pub(crate) fn prefix_number(num: u32, leading: u32) -> String {
    let length = (leading + 1) as usize;
    let mut num = num.to_string();
    while num.len() < length {
        num = format!("0{}", num);
    }
    num
}

async fn expand_numerical_mask(
    bot: &Bot,
    mask: &str,
    leading_zeros: u32,
) -> Result<Vec<Thread>> {
    println!("expanding {}", mask);
    let mut count = 1;
    let mut threads = vec![];
    // TODO: can we parallelize this?
    loop {
        let title = mask.replace("<#>", &prefix_number(count, leading_zeros));
        // println!("checking {}", &title);
        let code = match bot.get_page(&title).get_html().await {
            Ok(code) => code,
            Err(Error::PageDoesNotExist(_)) => {
                break;
            }
            Err(err) => return Err(err.into()),
        };
        threads.extend(extract_threads(&code)?);
        count += 1;
    }
    Ok(threads)
}

async fn expand_single_mask(bot: &Bot, title: &str) -> Result<Vec<Thread>> {
    println!("expanding single {}", title);
    match bot.get_page(title).get_html().await {
        Ok(code) => Ok(extract_threads(&code)?),
        Err(Error::PageDoesNotExist(_)) => Ok(vec![]),
        Err(err) => Err(err.into()),
    }
}

struct MonthMask {
    month: Month,
    year: u32,
}

impl MonthMask {
    fn incr(&mut self) {
        self.month = self.month.next();
        if self.month == Month::January {
            // Rolled over to next year
            self.year += 1;
        }
    }
}

async fn expand_monthly_mask(
    bot: &Bot,
    mask: &str,
    first_archive: &str,
) -> Result<Vec<Thread>> {
    println!("expanding {}", mask);
    let mut threads = vec![];
    // Turn mask into a regex to extract the first archive
    let regex = Regex::new(
        mask.replace("<month>", crate::MONTH_REGEX)
            .replace("<year>", r"(\d{4})")
            .as_str(),
    )
    .unwrap();
    let mut month_mask = match regex.captures(first_archive) {
        Some(capture) => {
            MonthMask {
                month: get_month(&capture[1]),
                // unwrap: Safe because regex matches \d{4}
                year: capture[2].parse().unwrap(),
            }
        }
        None => {
            return Err(anyhow!(
                "first_archive of {} does not match mask {}",
                first_archive,
                mask
            ));
        }
    };
    loop {
        let title = mask
            .replace("<month>", &month_mask.month.to_string())
            .replace("<year>", &month_mask.year.to_string())
            .to_string();
        let code = match bot.get_page(&title).get_html().await {
            Ok(code) => code,
            Err(Error::PageDoesNotExist(_)) => {
                break;
            }
            Err(err) => return Err(err.into()),
        };
        threads.extend(extract_threads(&code)?);
        month_mask.incr();
    }

    Ok(threads)
}

async fn expand_yearly_mask(
    bot: &Bot,
    mask: &str,
    first_archive: &str,
) -> Result<Vec<Thread>> {
    println!("expanding {}", mask);
    let mut threads = vec![];
    // Turn mask into a regex to extract the first archive
    let regex =
        Regex::new(mask.replace("<year>", r"(\d{4})").as_str()).unwrap();
    let mut year: u32 = match regex.captures(first_archive) {
        Some(capture) => {
            // unwrap: Safe because regex matches \d{4}
            capture[1].parse().unwrap()
        }
        None => {
            return Err(anyhow!(
                "first_archive of {} does not match mask {}",
                first_archive,
                mask
            ));
        }
    };
    loop {
        let title = mask.replace("<year>", &year.to_string()).to_string();
        let code = match bot.get_page(&title).get_html().await {
            Ok(code) => code,
            Err(Error::PageDoesNotExist(_)) => {
                break;
            }
            Err(err) => return Err(err.into()),
        };
        threads.extend(extract_threads(&code)?);
        year += 1;
    }

    Ok(threads)
}
