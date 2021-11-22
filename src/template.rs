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
use crate::thread::Thread;
use crate::Instructions;
use anyhow::Result;
use mwbot::Bot;

#[derive(Clone, Debug)]
pub(crate) struct Template {
    lead: String,
    header: String,
    row: String,
    altrow: Option<String>,
    footer: String,
    tail: String,
}

impl Template {
    pub(crate) fn build(
        &self,
        threads: Vec<Thread>,
        instructions: &Instructions,
    ) -> String {
        let masks: Vec<_> = instructions
            .masks
            .iter()
            .map(|mask| mask.to_string())
            .collect();
        let intro = format!(
            "Report generated based on a request from [[{}]]. It matches the following masks: {}.<br>",
            instructions.origin,
            masks.join(", ")
        );

        let mut text = vec![
            "<!-- HBC Archive Indexerbot can blank this -->".to_string(),
            self.lead.to_string(),
            intro,
            "{{last edited by}}".to_string(),
            self.header.to_string(),
        ];
        for (counter, thread) in threads.iter().enumerate() {
            let row = if counter % 2 == 0 {
                &self.row
            } else {
                self.altrow.as_ref().unwrap_or(&self.row)
            };
            let filled = row
                .replace("%%topic%%", &thread.topic)
                .replace("%%replies%%", &thread.replies.to_string())
                .replace("%%link%%", &thread.link)
                .replace("%%first%%", &thread.first())
                .replace("%%firstepoch%%", &thread.first_epoch().to_string())
                .replace("%%last%%", &thread.last())
                .replace("%%lastepoch%%", &thread.last_epoch().to_string())
                .replace("%%duration%%", &thread.duration())
                .replace(
                    "%%durationsecs%%",
                    &thread.duration_secs().to_string(),
                )
                .to_string();
            text.push(filled);
        }
        text.push(self.footer.to_string());
        text.push(self.tail.to_string());

        text.join("\n")
    }
}

enum State {
    None,
    Lead,
    Header,
    Row,
    Altrow,
    Footer,
    Tail,
}

pub(crate) async fn get_template(bot: &Bot, title: &str) -> Result<Template> {
    let page = bot.get_page(title);
    let wikitext = page.get_wikitext().await?;
    Ok(parse(&wikitext))
}

fn parse(text: &str) -> Template {
    let mut state = State::None;
    let mut lead = vec![];
    let mut header = vec![];
    let mut row = vec![];
    let mut altrow = vec![];
    let mut footer = vec![];
    let mut tail = vec![];
    for line in text.lines() {
        match line.trim() {
            "<!-- LEAD -->" => {
                state = State::Lead;
            }
            "<!-- HEADER -->" => {
                state = State::Header;
            }
            "<!-- ROW -->" => {
                state = State::Row;
            }
            "<!-- ALT ROW -->" => {
                state = State::Altrow;
            }
            "<!-- FOOTER -->" => {
                state = State::Footer;
            }
            "<!-- TAIL -->" => {
                state = State::Tail;
            }
            "<!-- END -->" => {
                break;
            }
            line => {
                match state {
                    State::None => {
                        // do nothing
                    }
                    State::Lead => {
                        lead.push(line.to_string());
                    }
                    State::Header => {
                        header.push(line.to_string());
                    }
                    State::Row => {
                        row.push(line.to_string());
                    }
                    State::Altrow => {
                        altrow.push(line.to_string());
                    }
                    State::Footer => {
                        footer.push(line.to_string());
                    }
                    State::Tail => {
                        tail.push(line.to_string());
                    }
                }
            }
        }
    }
    let altrow = if altrow.is_empty() {
        None
    } else {
        Some(altrow.join("\n"))
    };
    Template {
        lead: lead.join("\n"),
        header: header.join("\n"),
        row: row.join("\n"),
        altrow,
        footer: footer.join("\n"),
        tail: tail.join("\n"),
    }
}
