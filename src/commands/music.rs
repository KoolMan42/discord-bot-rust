use std::sync::atomic::Ordering;

use serenity::{
    client::Context,
    framework::{
        standard::{
            Args,
            CommandResult,
            macros::command,
        },
    },
    http::Http,
    model::{channel::Message, prelude::ChannelId},
    Result as SerenityResult,
};
use songbird::{EventContext, EventHandler as VoiceEventHandler, input::restartable::Restartable};
use songbird::tracks::PlayMode::Play;
use tracing::error;

// Import the `Context` to handle commands.

async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let channel_id = guild
        .voice_states.get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

    let _handler = manager.join(guild_id, connect_to).await;

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(err) => {
            check_msg(msg.channel_id.say(&ctx.http, "Must provide a URL to a video or audio").await);
            error!("{:?}", err);

            return Ok(());
        }
    };

    if !url.starts_with("http") {
        check_msg(msg.channel_id.say(&ctx.http, "Must provide a valid URL").await);

        return Ok(());
    }

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;


    let user_channel_id = guild
        .voice_states.get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let bot_channel_id = guild
        .voice_states.get(&ctx.cache.current_user_id().await)
        .and_then(|voice_state| voice_state.channel_id);

    if user_channel_id.is_none() {
        check_msg(msg.reply(ctx, "Not in a voice channel").await);

        return Ok(());
    }

    if user_channel_id != bot_channel_id && bot_channel_id.is_none() {
        join(&ctx, msg).await?;
    }


    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();


    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        let source = match Restartable::ytdl(url, true).await {
            // let source = match Restartable::ytdl(url, false).await {
            // let source = match ytdl(url).await {
            Ok(source) => source,
            Err(why) => {
                println!("Err starting source: {:?}", why);

                check_msg(msg.channel_id.say(&ctx.http, "Error sourcing ffmpeg").await);

                return Ok(());
            }
        };
        handler.enqueue_source(source.into());
        handler.remove_all_global_events();

        // handler.add_global_event(
        //     Event::Periodic(Duration::from_secs(1), None),
        //     IdleHandler {
        //         http: ctx.http.clone(),
        //         manager,
        //         interaction: interaction.clone(),
        //         limit: 60 * 10,
        //         count: Default::default(),
        //     },
        // );


        // handler.play_source(source.into());
        check_msg(
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Added song🎶 to queue: position {}", handler.queue().len()),
                )
                .await,
        );

        println!("Queue size {:?}", handler.queue().len());
    } else {
        check_msg(msg.channel_id.say(&ctx.http, "Not in a voice channel to play in").await);
    }

    Ok(())
}

#[command]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            check_msg(msg.channel_id.say(&ctx.http, format!("Failed: {:?}", e)).await);
        }

        check_msg(msg.channel_id.say(&ctx.http, "Left voice channel").await);
    } else {
        check_msg(msg.reply(ctx, "Not in a voice channel").await);
    }

    Ok(())
}


#[command]
#[only_in(guilds)]
async fn current(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Some(handler_lock) = manager.get(guild_id) {
            let handler = handler_lock.lock().await;

            if let Some(current_source) = handler.queue().current() {
                let current_source = current_source.metadata().clone();


                let current_source_info_string = format!(
                    "Title: {:?}\nDuration: {:?}\nPosition: {:?}\n",
                    current_source.title,
                    current_source.duration,
                    current_source.track
                );

                check_msg(msg.channel_id.say(&ctx.http, current_source_info_string).await);
            } else {
                check_msg(msg.channel_id.say(&ctx.http, "No current source").await);
            }
        } else {
            check_msg(msg.channel_id.say(&ctx.http, "No current source").await);
        }
    } else {
        check_msg(msg.channel_id.say(&ctx.http, "Not in a voice channel").await);
    }

    Ok(())
}


#[command]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        println!("Queue size before skip {:?}", handler.queue().len());

        if let Some(current_source) = handler.queue().current() {
            match current_source.get_info().await.unwrap().playing.eq(&Play) {
                true => {
                    let _ = handler.queue().skip();
                    check_msg(
                        msg.channel_id
                            .say(
                                &ctx.http,
                                format!("Song skipped: {:?} in queue.", handler.queue().len() - 1),
                            )
                            .await,
                    );
                }
                _ => {
                    check_msg(msg.channel_id.say(&ctx.http, "Not playing get prank'd").await);
                }
            }
        }

        println!("Queue size after skip {:?}", handler.queue().len());
        // drop(manager);
        // drop(handler);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}


#[command]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx).await
        .expect("Songbird Voice client placed in at initialisation.").clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;

        handler.stop();
        // drop(manager);
        drop(handler);
        check_msg(msg.channel_id.say(&ctx.http, "Stopped").await);
    } else {
        check_msg(msg.channel_id.say(&ctx.http, "Not in a voice channel").await);
    }

    Ok(())
}


/// Checks that a message successfully sent; if not, then logs why to stdout.
fn check_msg(result: SerenityResult<Message>) {
    if let Err(why) = result {
        println!("Error sending message: {:?}", why);
    }
}