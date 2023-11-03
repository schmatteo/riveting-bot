use pyo3::prelude::*;
use crate::utils::prelude::*;
use twilight_model::http::permission_overwrite::{PermissionOverwrite, PermissionOverwriteType};
use twilight_model::gateway::payload::incoming::MessageCreate;
use twilight_model::guild::Permissions;
use twilight_model::channel::Channel;
use twilight_util::builder::embed::{self, EmbedFieldBuilder};

use crate::commands::prelude::*;

pub struct Battleships;

impl Battleships {
    pub fn command() -> impl Into<BaseCommand> {
        use crate::commands::builder::*;

        command("battleships", "Playe a game of battleships.")
            .attach(Self::slash)
            .option(user("user", "User to play against").required())
            .dm()
    }

    async fn slash(ctx: Context, req: SlashRequest) -> CommandResponse {
        let player1 = req.interaction.author_id().ok_or(CommandError::MissingArgs)?;
        let player2 = req.args.user("user")?.unwrap_id();
        let players = [player1, player2];
        let guild_id = req.interaction.guild_id.unwrap();
        let mut channels: Vec<Channel> = Vec::new();
        let ship_types = ["destroyer", "cruiser", "battleship"];

        let everyone_permission_overwrite = PermissionOverwrite { allow: (None), deny: (Some(Permissions::VIEW_CHANNEL)), id: (guild_id.cast()), kind: (PermissionOverwriteType::Role) };

        let code = include_str!("engine.py");
        let game: &Result<Py<PyAny>, PyErr> = &Python::with_gil(|py| -> PyResult<PyObject> {
            let module = PyModule::from_code(py, code, "engine.py", "engine")?;
            let game = module.getattr("MultiPlayerGame")?.call0();

            Ok(game.unwrap().to_object(py))
        });

        for (index, player) in players.iter().enumerate() {
          let channel = ctx.http.create_guild_channel(guild_id, &format!("Player {}", index + 1)).unwrap().send().await?;
          let player_permission_overwrite = PermissionOverwrite { allow: (Some(Permissions::VIEW_CHANNEL)), deny: (None), id: (player.cast()), kind: (PermissionOverwriteType::Member) };
          ctx.http.update_channel_permission(channel.id, &everyone_permission_overwrite).await?;
          ctx.http.update_channel_permission(channel.id, &player_permission_overwrite).await?;
          channels.push(channel);
        }

        for (index, channel) in channels.iter().enumerate() {
            // Get a board for the player and create an embed for it
            let board = Python::with_gil(|py| -> PyResult<String> {
                let board = game.as_ref().unwrap().getattr(py, format!("player{}", index + 1).as_str()).unwrap().call_method0(py, "get_stringified_board").unwrap();
    
                board.extract::<String>(py)
            });
            let embed = embed::EmbedBuilder::new()
                .title("Battleships")
                .color(0x9500a8)
                .field(EmbedFieldBuilder::new("Board", format!("Your board:\n```{}```", board.unwrap())))
                .build();
            ctx.http.create_message(channel.id).embeds(&[embed]).unwrap().await?;

            // TODO both players should be able to place ships at the same time
            // While the first player places their ships, the second one gets a message asking them to wait
            if index == 0 {
                let wait_embed = embed::EmbedBuilder::new().title("Please wait while the other player places his ships").build();
                ctx.http.create_message(channels[index + 1].id).embeds(&[wait_embed]).unwrap().await?;
            }
            
            for ship_type in ship_types {
                let place_embed = embed::EmbedBuilder::new()
                    .title(format!("Select a place for your **{}**", ship_type))
                    .field(EmbedFieldBuilder::new("Message format: `x, y, orientation [v/h]`", "e.g. 3, 2, h"))
                    .build();
                ctx.http.create_message(channel.id).embeds(&[place_embed]).unwrap().await?;
  
                let message = ctx.standby
                    .wait_for_message(channel.id, move |event: &MessageCreate| {
                        event.content.split(',').count() == 3
                    })
                    .await?;
                println!("{}", message.author.name);
            }
        }

        Ok(Response::clear(ctx, req))
    }
}
