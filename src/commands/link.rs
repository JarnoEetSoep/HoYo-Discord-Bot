use std::sync::Arc;
use std::time::Duration;

use hoyo_api::prelude::*;
use serenity::builder::CreateApplicationCommand;
use serenity::collector::modal_interaction_collector;
use serenity::futures::StreamExt;
use serenity::model::application::component::ButtonStyle;
use serenity::model::application::interaction::application_command::ApplicationCommandInteraction;
use serenity::model::application::interaction::message_component::MessageComponentInteraction;
use serenity::model::prelude::component::{ActionRowComponent::InputText, InputTextStyle};
use serenity::model::prelude::interaction::InteractionResponseType;
use serenity::model::Timestamp;
use serenity::prelude::Context;
use sqlx::Row;

pub async fn run(
    database: &sqlx::SqlitePool,
    command: &ApplicationCommandInteraction,
    ctx: Arc<Context>,
) {
    command
        .create_interaction_response(&ctx, |res| {
            res.kind(InteractionResponseType::ChannelMessageWithSource)
                .interaction_response_data(|msg| {
                    msg.embed(|e| {
                        e.title("Link account")
                            .description("Are you sure you want to link an account?")
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
                        btn.custom_id("link_account")
                            .label("Link")
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

        if action == "link_account" {
            link(database, interaction, ctx.clone()).await;
        }
    }
}

pub fn register(command: &mut CreateApplicationCommand) -> &mut CreateApplicationCommand {
    command
        .name("link")
        .description("Link your HoYoLab account")
}

pub async fn link(
    database: &sqlx::SqlitePool,
    interaction: Arc<MessageComponentInteraction>,
    ctx: Arc<Context>,
) {
    let discord_id = interaction.user.id.0.to_string();

    interaction.create_interaction_response(&ctx, |res| {
        res.kind(InteractionResponseType::ChannelMessageWithSource)
            .interaction_response_data(|msg| {
                msg.embed(|e| {
                    e.description("Please login to <https://www.hoyolab.com/>, and write```js\njavascript:document.write(document.cookie)```in the URL bar. Then copy-paste this text into the input field you can open by pressing \"Continue\" below. Furthermore, you must type your in-game UID into the designated text input field.")
                        .colour((122, 71, 72))
                })
            })
    }).await.unwrap();

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
                            .label("Continue")
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
            interaction
                .create_interaction_response(&ctx, |res| {
                    res.kind(InteractionResponseType::Modal)
                        .interaction_response_data(|msg| {
                            msg.title("Link form")
                                .custom_id("link_form")
                                .components(|comp| {
                                    comp.create_action_row(|row| {
                                        row.create_input_text(|input| {
                                            input
                                                .custom_id("genshin_uid")
                                                .label("Genshin UID:")
                                                .placeholder("123456789")
                                                .style(InputTextStyle::Short)
                                        })
                                    })
                                    .create_action_row(
                                        |row| {
                                            row.create_input_text(|input| {
                                                input
                                                    .custom_id("cookie")
                                                    .label("HoYoLab Token:")
                                                    .style(InputTextStyle::Paragraph)
                                            })
                                        },
                                    )
                                })
                        })
                })
                .await
                .unwrap();

            let mut form_submit =
                modal_interaction_collector::ModalInteractionCollectorBuilder::new(&*ctx).build();

            while let Some(submission) = form_submit.next().await {
                if let InputText(genshin_uid) = submission
                    .data
                    .components
                    .get(0)
                    .unwrap()
                    .components
                    .get(0)
                    .unwrap()
                {
                    if let InputText(hoyo_cookie) = submission
                        .data
                        .components
                        .get(1)
                        .unwrap()
                        .components
                        .get(0)
                        .unwrap()
                    {
                        let hoyo_cookie = &hoyo_cookie.value;
                        let genshin_uid = &genshin_uid.value;

                        let destructured_cookie = Client::destructure_cookie(hoyo_cookie);

                        if let Err(error) = &destructured_cookie {
                            interaction
                                .create_interaction_response(&ctx, |res| {
                                    res.kind(InteractionResponseType::ChannelMessageWithSource)
                                        .interaction_response_data(|msg| {
                                            msg.content(format!(
                                                "Could not link account:\n{}",
                                                error
                                            ))
                                        })
                                })
                                .await
                                .unwrap();

                            return;
                        }

                        let (cookie, ltuid, ltoken, cookie_token, account_id, lang) =
                            destructured_cookie.unwrap();

                        submission.create_interaction_response(&ctx, |res| {
                            res.kind(InteractionResponseType::ChannelMessageWithSource)
                                .interaction_response_data(|msg| {
                                    msg.embed(|e| {
                                        e.description(format!("The following account will be linked:\nGenshin UID:```rust\n{}```HoYoLab cookie:```properties\n{}```", genshin_uid, cookie.split(" ").map(|cookie| cookie.replace("=", " = ")).collect::<Vec<String>>().join("\n")))
                                            .colour((122, 71, 72))
                                    })
                                })
                        }).await.unwrap();

                        let msg =
                            submission
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
                                                    .label("Link!")
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

                                if count_config == 0 {
                                    sqlx::query(
                                        format!(
                                            "INSERT INTO config (genshin_uid) VALUES (\"{}\");",
                                            genshin_uid
                                        )
                                        .as_str(),
                                    )
                                    .execute(database)
                                    .await
                                    .unwrap();
                                }

                                let count_hoyo_cookie = sqlx::query(format!("SELECT COUNT(*) FROM users INNER JOIN hoyo_cookie on users.hoyo_cookie_id = hoyo_cookie.cookie_id WHERE genshin_uid = \"{}\";", genshin_uid).as_str())
                                    .fetch_one(database).await.unwrap().get::<u32, _>(0);

                                if count_hoyo_cookie == 0 {
                                    sqlx::query(format!("INSERT INTO hoyo_cookie (ltuid, ltoken, cookie_token, account_id, lang) VALUES (\"{}\", \"{}\", \"{}\", \"{}\", \"{}\");", ltuid, ltoken, cookie_token, account_id, lang).as_str())
                                        .execute(database).await.unwrap();
                                }

                                let cookie_id = sqlx::query(
                                    format!(
                                        "SELECT cookie_id FROM hoyo_cookie WHERE ltuid = \"{}\";",
                                        ltuid
                                    )
                                    .as_str(),
                                )
                                .fetch_one(database)
                                .await
                                .unwrap()
                                .get::<u32, _>(0);

                                let query = sqlx::query(format!("INSERT INTO users (discord_id, hoyo_cookie_id, genshin_uid) VALUES (\"{}\", {}, \"{}\");", discord_id, cookie_id, genshin_uid).as_str())
                                    .execute(database).await;

                                if let Err(e) = query {
                                    interaction
                                        .create_interaction_response(&ctx, |res| {
                                            res.kind(
                                                InteractionResponseType::ChannelMessageWithSource,
                                            )
                                            .interaction_response_data(|msg| {
                                                msg.content(format!(
                                                    "Could not link account:\n{}",
                                                    e
                                                ))
                                            })
                                        })
                                        .await
                                        .unwrap();

                                    return;
                                }

                                interaction
                                    .create_interaction_response(&ctx, |res| {
                                        res.kind(InteractionResponseType::ChannelMessageWithSource)
                                            .interaction_response_data(|msg| {
                                                msg.content("Successfully linked account!")
                                            })
                                    })
                                    .await
                                    .unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
}
