use config::Config;
use teloxide::{
    prelude::*,
    utils::command::BotCommands,
    types::KeyboardButton,
    types::ParseMode,
    types::ReplyMarkup
};

const VERSION: Option<&str> = option_env!("CARGO_PKG_VERSION");
const REPOSITORY: Option<&str> = option_env!("CARGO_PKG_REPOSITORY");

#[tokio::main]
async fn main() {
    // Default log level to info if not set
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();
    log::info!("Starting wol bot...");

    // Load token from config
    let settings_result = Config::builder()
        .add_source(config::File::with_name("config"))
        .build();
    if settings_result.is_err() {
        log::error!("Failed to load config file. Please make sure it exists and is valid.");
        std::process::exit(1);
    }
    let settings = settings_result.unwrap();

    let token = settings.get_string("token").unwrap();
    if token == "YOUR_BOT_TOKEN" {
        log::error!("Token not set in config.toml");
        std::process::exit(1);
    }
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
            // Build repository inline link if available
            let mut repo_string = String::from("tgwol");
            if REPOSITORY.is_some() {
                repo_string = format!("<a href=\"{}\">tgwol</a>", REPOSITORY.unwrap().to_string());
            }

            // Send a message listing all the commands
            bot.send_message(msg.chat.id,
                Command::descriptions().to_string()
                + &format!("\n\n{repo_string} v{}", VERSION.unwrap_or("Unknown"))
            )
                .parse_mode(ParseMode::Html)
                .await?
        }
        Command::Wake(device) => {
            // If no device is specified, send an error message
            if device == "" {
                bot.send_message(msg.chat.id, "Please specify a device, e.g.\n<code>/wake mydevice</code>")
                    .parse_mode(ParseMode::Html)
                    .await?
            } else {
                // Load device configuration
                let settings = Config::builder()
                    .add_source(config::File::with_name("config"))
                    .build()
                    .unwrap();

                let mac_address_result = settings.get_string(&format!("devices.{}.mac", device));
                let telegram_id = settings.get_int(&format!("devices.{}.telegram_id", device));
                let incoming_id = msg.from().unwrap().id.0 as i64;
                // Check if device's mac address and authorized user id is configured correctly
                if mac_address_result.is_err() || telegram_id.is_err() {
                    bot.send_message(
                        msg.chat.id,
                        format!("Device \"{device}\" is not correctly configured."),
                    )
                    .await?
                } else if !vec![incoming_id, 0 as i64].contains(&telegram_id.unwrap())
                // Block user from waking device if telegram_id isn't 0 or doesn't match the user's id
                {
                    log::info!("Unauthorized user {incoming_id} tried to wake {device}");
                    bot.send_message(
                        msg.chat.id,
                        format!("You ({incoming_id}) are not authorized to wake {device}."),
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
                    let wol = wakey::WolPacket::from_string(&mac_address, mac_separator).unwrap();
                    log::debug!(
                        "Sending magic packet to {device} ({mac_address})...",
                        device = device,
                        mac_address = mac_address
                    );
                    if wol.send_magic().is_ok() {
                        // Success
                        bot.send_message(msg.chat.id, format!("Sent magic packet to {device}!"))
                            .reply_markup(ReplyMarkup::keyboard(vec![
                                vec![KeyboardButton::new(format!("/wake {device}"))],
                                ]))
                            .await?
                    } else {
                        // wakey gave an error
                        bot.send_message(
                            msg.chat.id,
                            format!("There was a problem waking {device}."),
                        )
                        .await?
                    }
                }
            }
        }
    };

    Ok(())
}
