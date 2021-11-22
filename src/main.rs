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

use anyhow::{anyhow, Result};
use mwbot::parsoid::*;
use mwbot::{generators, Bot, Error, Page, SaveOptions};
use parsoid::immutable::ImmutableWikicode;
use std::sync::Arc;

mod mask;
mod parser;
mod template;
mod thread;

use crate::mask::Mask;
use crate::template::Template;
type DefaultTemplate = Arc<Template>;

const MONTH_REGEX: &str = "(January|February|March|April|May|June|July|August|September|October|November|December)";

#[derive(Debug)]
struct Instructions {
    origin: String,
    target: String,
    masks: Vec<Mask>,
    template: String,
}

fn has_comment(code: ImmutableWikicode) -> bool {
    code.to_mutable().filter_comments().iter().any(|comment| {
        [
            "HBC Archive Indexerbot can blank this",
            "Legobot can blank this",
        ]
        .contains(&comment.text().trim())
    })
}

async fn follow_instructions(
    bot: &Bot,
    instructions: Instructions,
    default_template: DefaultTemplate,
) -> Result<()> {
    let target = bot.get_page(&instructions.target);
    let (target_code, redirect) = match target.get_html().await {
        Ok(code) => {
            let redirect = code.get_redirect().map(|redir| redir.target());
            (code.to_immutable(), redirect)
        }
        Err(Error::PageDoesNotExist(_)) => {
            return Err(anyhow!(
                "[[{}]]: target [[{}]] does not exist.",
                &instructions.origin,
                target.title()
            ));
        }
        Err(err) => return Err(err.into()),
    };
    let (target, target_code) = if let Some(redirect) = redirect {
        let target = bot.get_page(&redirect);
        let target_code = target.get_html().await?.to_immutable();
        (target, target_code)
    } else {
        (target, target_code)
    };
    if !has_comment(target_code) {
        return Err(anyhow!(
            "[[{}]]: target ([[{}]]) missing safe string",
            &instructions.origin,
            target.title()
        ));
    }
    // TODO: can we identify duplicates from overlapping masks?
    let mut threads = vec![];
    for mask in &instructions.masks {
        threads.extend(mask.expand(bot).await?);
    }
    threads.sort_by(|a, b| a.first.cmp(&b.first));
    if threads.is_empty() {
        return Err(anyhow!(
            "[[{}]]: found 0 threads, misconfiguration?",
            &instructions.origin
        ));
    }
    let new_wikitext = if instructions.template
        == "User:HBC Archive Indexerbot/default template"
    {
        default_template.build(threads, &instructions)
    } else {
        match template::get_template(bot, &instructions.template).await {
            Ok(template) => template.build(threads, &instructions),
            Err(_) => {
                // TODO: log this error
                default_template.build(threads, &instructions)
            }
        }
    };
    let old_wikitext = target.get_wikitext().await?;
    if old_wikitext.trim() == new_wikitext.trim() {
        // No changes needed!
        println!("No changes needed for [[{}]]", target.title());
        return Ok(());
    }
    target
        .save(new_wikitext, &SaveOptions::summary("Bot: Updating index"))
        .await?;
    println!("Saved [[{}]]", target.title());
    Ok(())
}

async fn handle_page(
    bot: Bot,
    page: Page,
    default_template: DefaultTemplate,
) -> Result<()> {
    let instructions = parser::parse_instructions(&page).await?;
    dbg!(&instructions);
    follow_instructions(&bot, instructions, default_template).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let bot = Bot::from_default_config().await.unwrap();
    let default_template = Arc::new(
        template::get_template(
            &bot,
            "User:HBC Archive Indexerbot/default template",
        )
        .await?,
    );
    /*handle_page(
        bot.clone(),
        bot.get_page("User talk:Legoktm"),
        default_template.clone(),
    )
    .await
    .unwrap();*/

    let mut handles = vec![];
    let mut stream =
        generators::embeddedin(&bot, "User:HBC Archive Indexerbot/OptIn");
    while let Some(page) = stream.recv().await {
        let page = page?;
        let bot = bot.clone();
        let default_template = default_template.clone();
        handles.push(tokio::spawn(async move {
            handle_page(bot, page, default_template).await
        }));
    }
    let mut errors = vec![];
    for handle in handles {
        match handle.await {
            Ok(Ok(_)) => {}
            Ok(Err(err)) => errors.push(err.to_string()),
            Err(err) => println!("join error: {}", err),
        }
    }
    println!(
        "{}",
        errors
            .iter()
            .map(|error| format!("* {}", error))
            .collect::<Vec<_>>()
            .join("\n")
    );
    Ok(())
}
