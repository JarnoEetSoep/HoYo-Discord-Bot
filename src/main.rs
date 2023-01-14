mod commands;

use std::env;
use std::sync::Arc;

use serenity::async_trait;
use serenity::model::application::command::Command;
use serenity::model::application::interaction::Interaction;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

struct Bot {
    database: sqlx::SqlitePool
}

#[async_trait]
impl EventHandler for Bot {
    async fn interaction_create(&self, ctx: Context, interaction: Interaction) {
        if let Interaction::ApplicationCommand(command) = interaction {
            let ctx = Arc::new(ctx);
            
            match command.data.name.as_str() {
                "claimcode" => commands::claim_code::run(&self.database, &command, ctx.clone()).await,
                "claimdaily" => commands::claim_daily::run(&self.database, &command, ctx.clone()).await,
                "accounts" => commands::accounts::run(&self.database, &command, ctx.clone()).await,
                "link" => commands::link::run(&self.database, &command, ctx.clone()).await,
                "unlink" => commands::unlink::run(&self.database, &command, ctx.clone()).await,
                "submitcode" => commands::submitcode::run(&self.database, &command, ctx.clone()).await,
                _ => ()
            }
        }
    }

    async fn ready(&self, ctx: Context, ready: Ready) {
        println!("{} is connected!", ready.user.name);
        
        Command::create_global_application_command(&ctx.http, |cmd| commands::claim_code::register(cmd)).await.unwrap();
        Command::create_global_application_command(&ctx.http, |cmd| commands::claim_daily::register(cmd)).await.unwrap();
        Command::create_global_application_command(&ctx.http, |cmd| commands::accounts::register(cmd)).await.unwrap();
        Command::create_global_application_command(&ctx.http, |cmd| commands::link::register(cmd)).await.unwrap();
        Command::create_global_application_command(&ctx.http, |cmd| commands::unlink::register(cmd)).await.unwrap();
        Command::create_global_application_command(&ctx.http, |cmd| commands::submitcode::register(cmd)).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    // Configure the client with your Discord bot token in the environment.
    dotenv::dotenv().expect("No .env file");

    let database = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(sqlx::sqlite::SqliteConnectOptions::new()
            .filename("database.sqlite")
            .create_if_missing(true)).await.expect("Could not connect to database");
    
    sqlx::migrate!().run(&database).await.unwrap();

    let bot = Bot {
        database
    };

    let token = env::var("DISCORD_TOKEN").expect("Expected a token in the environment");

    // Build our client.
    let mut client = Client::builder(token, GatewayIntents::DIRECT_MESSAGES)
        .event_handler(bot)
        .await
        .expect("Error creating client");

    client.start().await.expect("Error running bot.");
}