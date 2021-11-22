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
use crate::{Instructions, Mask};
use anyhow::{anyhow, Result};
use mwbot::parsoid::*;
use mwbot::Page;

/// Handle people writing |indexhere=<yes>
fn is_yes(text: &str) -> bool {
    ["<yes>", "yes"].contains(&text.to_lowercase().as_str())
}

fn prefix(text: &str, title: &str) -> String {
    if text.starts_with('/') {
        format!("{}{}", title, text)
    } else {
        text.to_string()
    }
}

fn parse_leading_zeros(input: Option<String>) -> u32 {
    match input {
        Some(input) => input.parse().unwrap_or(0),
        None => 0,
    }
}

fn parse_mask(input: &str, page: &Page, temp: &Template) -> Result<Mask> {
    if input.contains("<#>") {
        Ok(Mask::Numerical {
            mask: prefix(input, page.title()),
            leading_zeros: parse_leading_zeros(temp.get_param("leading_zeros")),
        })
    } else if input.ends_with("<#") {
        // Seems to be a common error of people omitting the final >
        Ok(Mask::Numerical {
            mask: prefix(&format!("{}>", input), page.title()),
            leading_zeros: parse_leading_zeros(temp.get_param("leading_zeros")),
        })
    } else if input.contains("<year>") {
        let first_archive = match temp.get_param("first_archive") {
            Some(first) => first,
            None => {
                return Err(anyhow!(
                    "[[{}]]: Missing |first_archive=",
                    page.title()
                ))
            }
        };
        let mask = prefix(input, page.title());
        if input.contains("<month>") {
            Ok(Mask::Monthly {
                mask,
                first_archive,
            })
        } else {
            Ok(Mask::Yearly {
                mask,
                first_archive,
            })
        }
    } else if !input.contains('<') {
        Ok(Mask::SinglePage {
            title: prefix(input, page.title()),
        })
    } else {
        Err(anyhow!(
            "[[{}]]: Unrecognized |mask= value: <nowiki>{}</nowiki>",
            page.title(),
            input
        ))
    }
}

pub(crate) async fn parse_instructions(page: &Page) -> Result<Instructions> {
    // println!("[[{}]]", page.title());
    let code = page.get_html().await?;
    for temp in code.filter_templates()? {
        if temp.name() == "User:HBC Archive Indexerbot/OptIn" {
            // Process the mask
            let mut masks = vec![];
            if let Some(input) = temp.get_param("mask") {
                masks.push(parse_mask(&input, page, &temp)?);
            }
            let mut counter = 1;
            while let Some(input) = temp.get_param(&format!("mask{}", counter))
            {
                masks.push(parse_mask(&input, page, &temp)?);
                counter += 1;
            }
            let target = match temp.get_param("target") {
                Some(target) => prefix(&target, page.title()),
                // Default target
                None => prefix("/Archive index", page.title()),
            };
            let template = match temp.get_param("template") {
                Some(temp) => {
                    if temp == "template location" || temp.is_empty() {
                        "User:HBC Archive Indexerbot/default template"
                            .to_string()
                    } else {
                        temp
                    }
                }
                None => {
                    "User:HBC Archive Indexerbot/default template".to_string()
                }
            };
            let indexhere = temp
                .get_param("indexhere")
                .map(|val| is_yes(&val))
                .unwrap_or(false);
            if masks.is_empty() {
                // Default mask
                masks.push(Mask::Numerical {
                    mask: prefix("/Archive <#>", page.title()),
                    leading_zeros: 0,
                });
            }
            // Now convert indexhere into a mask (needs to be after the empty check)
            if indexhere {
                masks.push(Mask::SinglePage {
                    title: page.title().to_string(),
                });
            }
            let instructions = Instructions {
                origin: page.title().to_string(),
                target,
                masks,
                template,
            };
            return Ok(instructions);
        }
    }
    // Unable to find, maybe wrapped in another template
    Err(anyhow!(
        "[[{}]]: Unable to find configuration, maybe wrapped in a template?",
        page.title()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_yes() {
        assert!(is_yes("yes"));
        assert!(is_yes("<yes>"));
        assert!(!is_yes("no"));
        assert!(!is_yes("<no>"));
    }

    #[test]
    fn test_prefix() {
        assert_eq!(&prefix("/Index", "Title"), "Title/Index");
        assert_eq!(&prefix("Foo index", "Title"), "Foo index");
    }
}
