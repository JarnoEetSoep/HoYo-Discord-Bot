use std::sync::Arc;
use std::time::Duration;

use serenity::builder::CreateApplicationCommand;
use serenity::futures::StreamExt;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::component::ButtonStyle;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::model::Timestamp;
use serenity::prelude::Context;
use sqlx::Row;

use crate::commands;

pub async fn run(
    database: &sqlx::SqlitePool,
    command: &ApplicationCommandInteraction,
    ctx: Arc<Context>,
) {
    let discord_id = command.user.id.0.to_string();

    let users = sqlx::query(
        format!(
            "SELECT genshin_uid FROM users WHERE discord_id = {}",
            discord_id
        )
        .as_str(),
    )
    .fetch_all(database)
    .await
    .unwrap();

    let genshin_ids = users
        .into_iter()
        .map::<String, _>(|user| user.get(0))
        .collect::<Vec<String>>();
    let genshin_names = vec!["TBA".to_string(); genshin_ids.len()];

    command
        .create_interaction_response(&ctx.http, |response| {
            response
                .kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| {
                    msg.embed(|e| {
                        e.title("Accounts")
                            .description("**These are your linked accounts:**")
                            .colour((122, 71, 72))
                            .field("Genshin Name", genshin_names.join("\n"), true)
                            .field("Genshin UID", genshin_ids.join("\n"), true)
                            .footer(|f| {
                                f.icon_url(command.user.avatar_url().unwrap()).text(format!(
                                    "Requested by {}#{}",
                                    command.user.name, command.user.discriminator
                                ))
                            })
                            .timestamp(Timestamp::now())
                    })
                })
        })
        .await
        .unwrap();

    let msg = command
        .user
        .direct_message(&ctx.http, |msg| {
            msg.components(|comp| {
                comp.create_action_row(|row| {
                    row.create_button(|btn| {
                        btn.custom_id("link_account_button")
                            .label("Link account")
                            .style(ButtonStyle::Success)
                    })
                    .create_button(|btn| {
                        btn.custom_id("unlink_account_button")
                            .label("Unlink account")
                            .style(ButtonStyle::Danger)
                            .disabled(genshin_ids.len() == 0)
                    })
                })
            })
        })
        .await
        .unwrap();

    let mut interaction_stream = msg
        .await_component_interactions(&*ctx)
        .timeout(Duration::from_secs(120))
        .build();

    while let Some(interaction) = interaction_stream.next().await {
        let action = &interaction.data.custom_id;

        msg.delete(&ctx).await.unwrap();

        if action == "link_account_button" {
            commands::link::link(database, interaction, ctx.clone()).await;
        } else {
            commands::unlink::unlink(database, interaction, ctx.clone()).await;
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("accounts")
        .description("List your linked accounts and (un)link accounts.")
}
