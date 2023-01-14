use std::sync::Arc;

use hoyo_api::prelude::*;
use serenity::builder::CreateApplicationCommand;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::prelude::Context;
use sqlx::Row;

pub async fn run(database: &sqlx::SqlitePool, command: &ApplicationCommandInteraction, ctx: Arc<Context>) {
    let discord_id = command.user.id.0.to_string();

    let users = sqlx::query(format!("SELECT DISTINCT ltuid, ltoken, cookie_token, account_id, lang, genshin_uid FROM users \
                                                         INNER JOIN hoyo_cookie on users.hoyo_cookie_id = hoyo_cookie.cookie_id \
                                                         WHERE discord_id = \"{}\";", discord_id).as_str())
        .fetch_all(database).await.unwrap();

    if users.len() == 0 {
        command.create_interaction_response(&ctx.http, |response| {
            response.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| msg.content("You have no linked accounts"))
        }).await.unwrap();

        return;
    }

    let mut buffer = Vec::<String>::new();

    for user in users {
        let hoyo_cookie = Cookie::CookieParsed(user.get(0), user.get(1), user.get(2), user.get(3), user.get(4));
        let genshin_uid: String = user.get(5);

        let hoyo_client = Client::new(hoyo_cookie, genshin_uid.as_str());

        if let Err(error) = hoyo_client {
            command.create_interaction_response(&ctx.http, |response| {
                response.kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|msg| msg.content(format!("Could not connect hoyo client: {}", error)))
            }).await.unwrap();

            return;
        }

        let hoyo_client = hoyo_client.unwrap();

        let output = tokio::task::spawn_blocking(move || {            
            if let Err(error) = hoyo_client.claim_daily() {
                format!("Error claiming daily on {}: `{}`", genshin_uid, error)
            } else {
                format!("Successfully claimed daily on {}", genshin_uid)
            }
        }).await.unwrap();

        buffer.push(output);
    }

    command.create_interaction_response(&ctx.http, |response| {
        response.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|msg| msg.content(buffer.join("\n")))
    }).await.unwrap();
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command.name("claimdaily").description("Claim daily login reward")
}