use config::Config;
use teloxide::{prelude::*, utils::command::BotCommands};

#[tokio::main]
async fn main() {
    // Default log level to info if not set
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
    log::info!("Starting wol bot...");

    // Load token from config
    let settings = Config::builder()
        .add_source(config::File::with_name("config"))
        .build()
        .unwrap();
    let token = settings.get_string("token").unwrap();
    // Create instance of bot
    let bot = Bot::new(token);
    Command::repl(bot, answer).await;
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    // Define functional commands
    #[command(description = "display this text.")]
    Help,
    #[command(description = "wake a device.")]
    Wake(String),
}

async fn answer(bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
    match cmd {
        Command::Help => {
            // Send a message listing all the commands
            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                .await?
        }
        Command::Wake(device) => {
            // If no device is specified, send an error message
            if device == "" {
                bot.send_message(msg.chat.id, "Please specify a device.")
                    .await?
            } else {
                // Load device configuration
                let settings = Config::builder()
                    .add_source(config::File::with_name("config"))
                    .build()
                    .unwrap();

                let mac_address_result = settings.get_string(&format!("devices.{}.mac", device));
                let telegram_id = settings.get_int(&format!("devices.{}.telegram_id", device));
                // Check if device's mac address and authorized user id is configured correctly
                if mac_address_result.is_err() || telegram_id.is_err() {
                    bot.send_message(
                        msg.chat.id,
                        format!("Device \"{device}\" not correctly configured."),
                    )
                    .await?
                } else if vec![msg.from().unwrap().id.0 as i64, 0 as i64]
                    .contains(&telegram_id.unwrap())
                // Allow user to wake device if telegram_id is 0 or matches the user's id
                {
                    bot.send_message(
                        msg.chat.id,
                        format!("You are not authorized to wake {device}."),
                    )
                    .await?
                } else {
                    let mac_address = mac_address_result.unwrap();
                    // Load mac address separator from config and convert to char
                    let mac_separator = settings
                        .get_string("mac_separator")
                        .unwrap()
                        .chars()
                        .next()
                        .unwrap();

                    // Setup wol packet and send
                    let wol = wakey::WolPacket::from_string(&mac_address, mac_separator);
                    log::info!(
                        "Waking {device} ({mac_address})...",
                        device = device,
                        mac_address = mac_address
                    );
                    if wol.send_magic().is_ok() {
                        // Success
                        bot.send_message(msg.chat.id, format!("Waking {device}..."))
                            .await?
                    } else {
                        // wakey gave an error
                        bot.send_message(
                            msg.chat.id,
                            format!("There was a problem waking {device}..."),
                        )
                        .await?
                    }
                }
            }
        }
    };

    Ok(())
}
