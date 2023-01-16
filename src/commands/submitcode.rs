use std::sync::Arc;

use hoyo_api::prelude::*;
use serenity::builder::CreateApplicationCommand;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::UserId;
use serenity::model::prelude::command::CommandOptionType;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::model::prelude::interaction::application_command::CommandDataOptionValue;
use serenity::prelude::Context;
use sqlx::Row;

pub async fn run(database: &sqlx::SqlitePool, command: &ApplicationCommandInteraction, ctx: Arc<Context>) {
    let discord_id = command.user.id.0.to_string();

    let usercount = sqlx::query(format!("SELECT COUNT(*) FROM users WHERE discord_id = \"{}\";", discord_id).as_str())
        .fetch_one(database).await.unwrap().get::<u32, _>(0);
    
    if usercount == 0 {
        command.create_interaction_response(&ctx.http, |response| {
            response.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| msg.content("You may only submit redemption codes if you have at least one linked account."))
        }).await.unwrap();

        return;
    }

    if let CommandDataOptionValue::String(code) = command.data.options.get(0).unwrap().resolved.as_ref().unwrap() {
        if sqlx::query(format!("SELECT COUNT(*) FROM codes WHERE code = \"{}\";", code).as_str()).fetch_one(database).await.unwrap().get::<u32, _>(0) != 0 {
            command.create_interaction_response(&ctx.http, |response| {
                response.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|msg| msg.content(format!("Code {} already exists in the system.", code)))
            }).await.unwrap();

            return;
        }

        let success = sqlx::query(format!("INSERT INTO codes (code) VALUES (\"{}\");", code).as_str())
            .execute(database).await;

        if let Err(err) = success {
            command.create_interaction_response(&ctx.http, |response| {
                response.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|msg| msg.content(format!("Error submitting code `{}`: {}", code, err)))
            }).await.unwrap();

            return;
        }

        command.create_interaction_response(&ctx.http, |response| {
            response.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| msg.content(format!("Submitted code {}!", code)))
        }).await.unwrap();

        // Claim code on all linked accounts
        let users = sqlx::query(format!("SELECT DISTINCT ltuid, ltoken, cookie_token, account_id, lang, users.genshin_uid FROM users \
                                                              INNER JOIN hoyo_cookie on users.hoyo_cookie_id = hoyo_cookie.cookie_id \
                                                              INNER JOIN config on users.genshin_uid = config.genshin_uid \
                                                              WHERE config.auto_claim_codes = 1;").as_str())
            .fetch_all(database).await.unwrap();

        for user in users {
            let hoyo_cookie = Cookie::CookieParsed(user.get(0), user.get(1), user.get(2), user.get(3), user.get(4));
            let genshin_uid: String = user.get(5);

            let hoyo_client = Client::new(hoyo_cookie, genshin_uid.as_str());

            if let Err(error) = hoyo_client {
                command.user.direct_message(&ctx.http, |msg| {
                    msg.content(format!("Could not connect hoyo client: {}", error))
                }).await.unwrap();

                return;
            }

            let code = code.clone();
            let uid = genshin_uid.clone();

            let output = tokio::task::spawn_blocking(move || {
                if let Err(error) = hoyo_client.unwrap().claim_code(&code) {
                    format!("Error auto-claiming code `{}` on {}: `{}`", code, uid, error)
                } else {
                    format!("Successfully auto-claimed code `{}` on {}", code, uid)
                }
            }).await.unwrap();

            let discord_ids = sqlx::query(format!("SELECT discord_id FROM users WHERE genshin_uid = {};", genshin_uid).as_str())
                .fetch_all(database).await.unwrap().into_iter().map::<String, _>(|id| id.get(0)).collect::<Vec<String>>();
            
            for discord_id in discord_ids {
                let success = UserId(discord_id.trim().parse::<u64>().unwrap()).create_dm_channel(&ctx).await.unwrap().send_message(&ctx.http, |msg| {
                    msg.content(output.clone())
                }).await;

                if let Err(error) = success {
                    println!("Error sending confirmation to `{}`:\n {}", discord_id.trim(), error);
                }
            }
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("submitcode").description("Submit a redemption code")
        .create_option(|option| {
            option.name("code")
                .description("Redemption code")
                .kind(CommandOptionType::String)
                .required(true)
        })
}