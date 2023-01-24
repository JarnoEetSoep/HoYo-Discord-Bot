use std::sync::Arc;
use std::time::Duration;

use serenity::builder::{CreateApplicationCommand, CreateSelectMenuOption};
use serenity::futures::StreamExt;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::prelude::component::ButtonStyle;
use serenity::model::prelude::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::model::Timestamp;
use serenity::prelude::Context;
use sqlx::Row;

pub async fn run(
    database: &sqlx::SqlitePool,
    command: &ApplicationCommandInteraction,
    ctx: Arc<Context>,
) {
    let discord_id = command.user.id.0.to_string();

    let usercount = sqlx::query(
        format!(
            "SELECT COUNT(*) FROM users WHERE discord_id = \"{}\";",
            discord_id
        )
        .as_str(),
    )
    .fetch_one(database)
    .await
    .unwrap()
    .get::<u32, _>(0);

    if usercount == 0 {
        command
            .create_interaction_response(&ctx.http, |response| {
                response
                    .kind(InteractionResponseType::ChannelMessageWithSource)
                    .interaction_response_data(|msg| msg.content("You have no linked accounts"))
            })
            .await
            .unwrap();

        return;
    }

    command
        .create_interaction_response(&ctx, |res| {
            res.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| {
                    msg.embed(|e| {
                        e.description("Are you sure you want to unlink an account?")
                            .colour((122, 71, 72))
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
                        btn.custom_id("cancel")
                            .label("Cancel")
                            .style(ButtonStyle::Secondary)
                    })
                    .create_button(|btn| {
                        btn.custom_id("proceed")
                            .label("Unlink")
                            .style(ButtonStyle::Success)
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

        if action == "proceed" {
            unlink(database, interaction, ctx.clone()).await;
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("unlink")
        .description("Unlink your HoYoLab account")
}

pub async fn unlink(
    database: &sqlx::SqlitePool,
    interaction: Arc<MessageComponentInteraction>,
    ctx: Arc<Context>,
) {
    let discord_id = interaction.user.id.0.to_string();

    let users = sqlx::query(
        format!(
            "SELECT genshin_uid FROM users WHERE discord_id = {};",
            discord_id
        )
        .as_str(),
    )
    .fetch_all(database)
    .await
    .unwrap();

    let users = users
        .into_iter()
        .map::<String, _>(|user| user.get(0))
        .collect::<Vec<String>>();

    interaction
        .create_interaction_response(&ctx, |res| {
            res.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| {
                    msg.embed(|e| {
                        e.description("Please select an account to unlink")
                            .colour((122, 71, 72))
                    })
                })
        })
        .await
        .unwrap();

    let msg = interaction
        .user
        .direct_message(&ctx, |msg| {
            msg.components(|comp| {
                comp.create_action_row(|row| {
                    row.create_select_menu(|menu| {
                        menu.custom_id("account")
                            .placeholder("Select UID")
                            .options(|select| {
                                let mut options = Vec::new();

                                for user in &users {
                                    options.push(CreateSelectMenuOption::new(user, user));
                                }

                                select.set_options(options)
                            })
                    })
                })
            })
        })
        .await
        .unwrap();

    let interaction = match msg
        .await_component_interaction(&*ctx)
        .timeout(Duration::from_secs(120))
        .await
    {
        Some(val) => {
            msg.delete(&ctx).await.unwrap();

            if &interaction.data.custom_id == "cancel" {
                return;
            }

            val
        }
        None => {
            msg.reply(&ctx, "Timed out").await.unwrap();
            msg.delete(&ctx).await.unwrap();
            return;
        }
    };

    let genshin_uid = &interaction.data.values[0];

    interaction
        .create_interaction_response(&ctx, |res| {
            res.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| {
                    msg.embed(|e| {
                        e.description(format!(
                            "Your account with uid: `{}` will be unlinked. Proceed?",
                            genshin_uid
                        ))
                        .colour((122, 71, 72))
                    })
                })
        })
        .await
        .unwrap();

    let msg = interaction
        .user
        .direct_message(&ctx.http, |msg| {
            msg.components(|comp| {
                comp.create_action_row(|row| {
                    row.create_button(|btn| {
                        btn.custom_id("cancel")
                            .label("Cancel")
                            .style(ButtonStyle::Secondary)
                    })
                    .create_button(|btn| {
                        btn.custom_id("proceed")
                            .label("Unlink!")
                            .style(ButtonStyle::Success)
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

        if action == "proceed" {
            let count_config = sqlx::query(format!("SELECT COUNT(*) FROM users INNER JOIN config on users.genshin_uid = config.genshin_uid WHERE users.genshin_uid = \"{}\";", genshin_uid).as_str())
                .fetch_one(database).await.unwrap().get::<u32, _>(0);

            let hoyo_query = sqlx::query(format!("SELECT COUNT(*), cookie_id FROM users INNER JOIN hoyo_cookie on users.hoyo_cookie_id = hoyo_cookie.cookie_id WHERE genshin_uid = \"{}\";", genshin_uid).as_str())
                .fetch_one(database).await.unwrap();

            let count_hoyo_cookie: u32 = hoyo_query.get(0);

            let query = sqlx::query(
                format!(
                    "DELETE FROM users WHERE (discord_id, genshin_uid) = (\"{}\", \"{}\");",
                    discord_id, genshin_uid
                )
                .as_str(),
            )
            .execute(database)
            .await;

            if let Err(e) = query {
                interaction
                    .create_interaction_response(&ctx, |res| {
                        res.kind(InteractionResponseType::ChannelMessageWithSource)
                            .interaction_response_data(|msg| {
                                msg.content(format!("Could not unlink account:\n{}", e))
                            })
                    })
                    .await
                    .unwrap();

                return;
            }

            if count_config == 1 {
                let query = sqlx::query(
                    format!(
                        "DELETE FROM config WHERE genshin_uid = \"{}\";",
                        genshin_uid
                    )
                    .as_str(),
                )
                .execute(database)
                .await;

                if let Err(e) = query {
                    interaction
                        .create_interaction_response(&ctx, |res| {
                            res.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|msg| {
                                    msg.content(format!("Could not unlink account:\n{}", e))
                                })
                        })
                        .await
                        .unwrap();

                    return;
                }
            }

            if count_hoyo_cookie == 1 {
                let cookie_id: u32 = hoyo_query.get(1);

                let query = sqlx::query(
                    format!(
                        "DELETE FROM hoyo_cookie WHERE cookie_id = \"{}\";",
                        cookie_id
                    )
                    .as_str(),
                )
                .execute(database)
                .await;

                if let Err(e) = query {
                    interaction
                        .create_interaction_response(&ctx, |res| {
                            res.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|msg| {
                                    msg.content(format!("Could not unlink account:\n{}", e))
                                })
                        })
                        .await
                        .unwrap();

                    return;
                }
            }

            interaction
                .create_interaction_response(&ctx, |res| {
                    res.kind(InteractionResponseType::ChannelMessageWithSource)
                        .interaction_response_data(|msg| {
                            msg.content("Successfully unlinked account!")
                        })
                })
                .await
                .unwrap();
        }
    }
}
