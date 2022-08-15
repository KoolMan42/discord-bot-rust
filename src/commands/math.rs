use serenity::framework::standard::{Args, CommandError, CommandResult};
use serenity::framework::standard::macros::command;
use serenity::model::prelude::*;
use serenity::prelude::*;

#[command]
pub async fn multiply(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    if args.len() < 2 {
        return Err(CommandError::from(
            "Please provide at least two numbers to multiply.",
        ));
    }

    let temp1 = args.message();
    let temp2 = temp1.split_whitespace().collect::<String>();
    let temp3 = temp2.chars().all(char::is_numeric);


    if temp3 {
        let solution = args.iter::<f64>().map(|x| x.unwrap()).fold(1_f64, |acc, x| acc * x);

        msg.channel_id.say(&ctx.http, solution).await?;
    }


    Ok(())
}