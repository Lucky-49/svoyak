use crate::db;
use crate::db::{
    add_to_single_question_db, delete_game_player, establish_connection, get_player_profile,
    get_player_statistic, rec_pre_reg_player, rec_real_player_data_to_db, reg_game_player,
    reg_game_spectator, PlayerResultGame,
};
use crate::tg::token;
use rusqlite::params;
use std::error::Error;
use std::fmt::Debug;
use teloxide::dispatching::{dialogue, UpdateHandler};
use teloxide::prelude::Message;
use teloxide::types::{CallbackQuery, MessageId, ReplyMarkup};
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::utils::command::BotCommands;
use teloxide::{dispatching::dialogue::InMemStorage, prelude::*};

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn Error + Send + Sync>>;

//перечень состояний бота
#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    ReceivePlayerRealFirstName,
    ReceivePlayerRealPatronymic {
        player_real_first_name: String,
    },
    ReceivePlayerRealLastName {
        player_real_first_name: String,
        player_real_patronymic: String,
    },
    ReceivePlayerRealLocation {
        player_real_first_name: String,
        player_real_patronymic: String,
        player_real_last_name: String,
    },
    ReceivePlayerPhoneNumber {
        player_real_first_name: String,
        player_real_patronymic: String,
        player_real_last_name: String,
        player_real_location: String,
    },
    ConfirmationOfRegistration,
    RegistrationComplete,

    NumberQuestionFromPlayer,
    ReceiveSingleQuestionFromPlayer,
    ReceiveSingleAnswerQuestionFromPlayer {
        player_single_question: String,
    },
    ConfirmationSingleQuestionFromPlayer,
    ReceiveTopicMultiQuestionFromPlayer,
    ReceiveFirstQuestionFromPlayer {
        player_topic_multi_question: String,
    },
    ReceiveFirstAnswerQuestionFromPlayer {
        player_topic_multi_question: String,
        player_first_question: String,
    },
    ReceiveSecondQuestionFromPlayer {
        player_topic_multi_question: String,
        player_first_question: String,
        player_first_answer_question: String,
    },
    ReceiveSecondAnswerQuestionFromPlayer {
        player_topic_multi_question: String,
        player_first_question: String,
        player_first_answer_question: String,
        player_second_question: String,
    },
    ReceiveThirdQuestionFromPlayer {
        player_topic_multi_question: String,
        player_first_question: String,
        player_first_answer_question: String,
        player_second_question: String,
        player_second_answer_question: String,
    },
    ReceiveThirdAnswerQuestionFromPlayer {
        player_topic_multi_question: String,
        player_first_question: String,
        player_first_answer_question: String,
        player_second_question: String,
        player_second_answer_question: String,
        player_third_question: String,
    },
    ReceiveFourthQuestionFromPlayer {
        player_topic_multi_question: String,
        player_first_question: String,
        player_first_answer_question: String,
        player_second_question: String,
        player_second_answer_question: String,
        player_third_question: String,
        player_third_answer_question: String,
    },
    ReceiveFourthAnswerQuestionFromPlayer {
        player_topic_multi_question: String,
        player_first_question: String,
        player_first_answer_question: String,
        player_second_question: String,
        player_second_answer_question: String,
        player_third_question: String,
        player_third_answer_question: String,
        player_fourth_question: String,
    },
    ReceiveFifthQuestionFromPlayer {
        player_topic_multi_question: String,
        player_first_question: String,
        player_first_answer_question: String,
        player_second_question: String,
        player_second_answer_question: String,
        player_third_question: String,
        player_third_answer_question: String,
        player_fourth_question: String,
        player_fourth_answer_question: String,
    },
    ReceiveFifthAnswerQuestionFromPlayer {
        player_topic_multi_question: String,
        player_first_question: String,
        player_first_answer_question: String,
        player_second_question: String,
        player_second_answer_question: String,
        player_third_question: String,
        player_third_answer_question: String,
        player_fourth_question: String,
        player_fourth_answer_question: String,
        player_fifth_question: String,
    },
    ChoiceNumberQuestionFromPlayer,
    ConfirmationQuestionFromPlayer,
    RegistrationForTheGame,
}

//перечень команд обрабатываеые ботом
#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
enum Command {
    //комады которые поддерживает бот и которые будут отправлены юзеру при команде /help
    #[command(
        description = "Отмена (Также используется для прекращения отправки вопросов для игры)"
    )]
    Start,
    #[command(description = "Список команд")]
    Help,
    #[command(description = "Правила игры")]
    Rules,
    #[command(description = "Ваши регистрационные данные")]
    Playerdata, //Регистрационные данные юзера
    #[command(description = "Изменить регистрационные данные")]
    Rename, //Изменить регистрационные данные
    #[command(description = "Ваша статистика")]
    Statistics, //Статистика
    #[command(description = "Список предстоящих игр, регистрация на игру")]
    RegGame, //Регистрация на игру
    #[command(description = "Отправить вопрос для включения в игру")]
    Question, //Отправить вопрос для игры
}

#[derive(Debug)]
pub struct PlayerData {
    //структура данных юзера получаемых при подключении игрока к боту
    pub player_id: Option<i64>,
    pub player_name: Option<String>,
    pub player_first_name: Option<String>,
    pub player_last_name: Option<String>,
    pub chat_id: Option<ChatId>,
}

//структура ответов игроком на вопросы при знакомстве с ботом
pub struct RealPlayerData {
    pub player_real_first_name: String,
    pub player_real_patronymic: String,
    pub player_real_last_name: String,
    pub player_real_location: String,
    pub player_real_phone_number: i64,
}

//структура для отправки своих регистрационных данных по запросу игрока
pub struct PlayerProfile {
    pub player_id: Option<i64>,
    pub chat_id: Option<ChatId>,
    pub player_real_first_name: Option<String>,
    pub player_real_patronymic: Option<String>,
    pub player_real_last_name: Option<String>,
    pub player_real_location: Option<String>,
    pub player_real_phone_number: Option<i64>,
}

//структура одиночного вопроса отправляемого игроком в игру
pub struct PlayerSingleQuestion {
    pub player_id: Option<i64>,
    pub player_single_question: String,
    pub player_single_answer_question: String,
}

//структура пять вопросов одной темы
pub struct UserMultiQuestion {
    pub player_topic_multi_question: String,
    pub player_first_question: String,
    pub player_first_answer_question: String,
    pub player_second_question: String,
    pub player_second_answer_question: String,
    pub player_third_question: String,
    pub player_third_answer_question: String,
    pub player_fourth_question: String,
    pub player_fourth_answer_question: String,
    pub player_fifth_question: String,
    pub player_fifth_answer_question: String,
}

//запуск бота
pub async fn run_bot() {
    let token = token::TELEGRAM_TOKEN;

    let bot = Bot::new(token);

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn Error + Send + Sync + 'static>> {
    //эта функция регулирует переход бота из одного состояния диалога в другое
    //а так же мереход между диалогом и колбэком
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![State::Start].branch(case![Command::Start].endpoint(start))) //ввод игроком имени
        .branch(
            case![State::ReceivePlayerRealFirstName]
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Rules].endpoint(rules)),
        )
        .branch(
            case![State::ReceivePlayerRealPatronymic {
                player_real_first_name
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules)),
        )
        .branch(
            case![State::ReceivePlayerRealLastName {
                player_real_first_name,
                player_real_patronymic
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules)),
        )
        .branch(
            case![State::ReceivePlayerRealLocation {
                player_real_first_name,
                player_real_patronymic,
                player_real_last_name
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules)),
        )
        .branch(
            case![State::ReceivePlayerPhoneNumber {
                player_real_first_name,
                player_real_patronymic,
                player_real_last_name,
                player_real_location
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules)),
        )
        .branch(
            case![State::ConfirmationOfRegistration].branch(case![Command::Start].endpoint(start)),
        )
        .branch(
            case![State::ChoiceNumberQuestionFromPlayer]
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Rules].endpoint(rules))
                .branch(case![Command::Playerdata].endpoint(player_data))
                .branch(case![Command::Statistics].endpoint(statistic))
                .branch(case![Command::Rename].endpoint(rename))
                .branch(case![Command::RegGame].endpoint(reg_game))
                .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveSingleQuestionFromPlayer]
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Rules].endpoint(rules)),
        )
        .branch(
            case![State::ReceiveSingleAnswerQuestionFromPlayer {
                player_single_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules)),
        )
        .branch(
            case![State::ConfirmationSingleQuestionFromPlayer]
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Rules].endpoint(rules))
                .branch(case![Command::Playerdata].endpoint(player_data))
                .branch(case![Command::Statistics].endpoint(statistic))
                .branch(case![Command::Rename].endpoint(rename))
                .branch(case![Command::RegGame].endpoint(reg_game))
                .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveTopicMultiQuestionFromPlayer]
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Rules].endpoint(rules))
                .branch(case![Command::Playerdata].endpoint(player_data))
                .branch(case![Command::Statistics].endpoint(statistic))
                .branch(case![Command::Rename].endpoint(rename))
                .branch(case![Command::RegGame].endpoint(reg_game))
                .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveFirstQuestionFromPlayer {
                player_topic_multi_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules))
            .branch(case![Command::Playerdata].endpoint(player_data))
            .branch(case![Command::Statistics].endpoint(statistic))
            .branch(case![Command::Rename].endpoint(rename))
            .branch(case![Command::RegGame].endpoint(reg_game))
            .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveFirstAnswerQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules))
            .branch(case![Command::Playerdata].endpoint(player_data))
            .branch(case![Command::Statistics].endpoint(statistic))
            .branch(case![Command::Rename].endpoint(rename))
            .branch(case![Command::RegGame].endpoint(reg_game))
            .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveSecondQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules))
            .branch(case![Command::Playerdata].endpoint(player_data))
            .branch(case![Command::Statistics].endpoint(statistic))
            .branch(case![Command::Rename].endpoint(rename))
            .branch(case![Command::RegGame].endpoint(reg_game))
            .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveSecondAnswerQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules))
            .branch(case![Command::Playerdata].endpoint(player_data))
            .branch(case![Command::Statistics].endpoint(statistic))
            .branch(case![Command::Rename].endpoint(rename))
            .branch(case![Command::RegGame].endpoint(reg_game))
            .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveThirdQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules))
            .branch(case![Command::Playerdata].endpoint(player_data))
            .branch(case![Command::Statistics].endpoint(statistic))
            .branch(case![Command::Rename].endpoint(rename))
            .branch(case![Command::RegGame].endpoint(reg_game))
            .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveThirdAnswerQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules))
            .branch(case![Command::Playerdata].endpoint(player_data))
            .branch(case![Command::Statistics].endpoint(statistic))
            .branch(case![Command::Rename].endpoint(rename))
            .branch(case![Command::RegGame].endpoint(reg_game))
            .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveFourthQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question,
                player_third_answer_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules))
            .branch(case![Command::Playerdata].endpoint(player_data))
            .branch(case![Command::Statistics].endpoint(statistic))
            .branch(case![Command::Rename].endpoint(rename))
            .branch(case![Command::RegGame].endpoint(reg_game))
            .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveFourthAnswerQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question,
                player_third_answer_question,
                player_fourth_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules))
            .branch(case![Command::Playerdata].endpoint(player_data))
            .branch(case![Command::Statistics].endpoint(statistic))
            .branch(case![Command::Rename].endpoint(rename))
            .branch(case![Command::RegGame].endpoint(reg_game))
            .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveFifthQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question,
                player_third_answer_question,
                player_fourth_question,
                player_fourth_answer_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules))
            .branch(case![Command::Playerdata].endpoint(player_data))
            .branch(case![Command::Statistics].endpoint(statistic))
            .branch(case![Command::Rename].endpoint(rename))
            .branch(case![Command::RegGame].endpoint(reg_game))
            .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::ReceiveFifthAnswerQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question,
                player_third_answer_question,
                player_fourth_question,
                player_fourth_answer_question,
                player_fifth_question
            }]
            .branch(case![Command::Start].endpoint(start))
            .branch(case![Command::Help].endpoint(help))
            .branch(case![Command::Rules].endpoint(rules))
            .branch(case![Command::Playerdata].endpoint(player_data))
            .branch(case![Command::Statistics].endpoint(statistic))
            .branch(case![Command::Rename].endpoint(rename))
            .branch(case![Command::RegGame].endpoint(reg_game))
            .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::RegistrationComplete]
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Rules].endpoint(rules))
                .branch(case![Command::Playerdata].endpoint(player_data))
                .branch(case![Command::Statistics].endpoint(statistic))
                .branch(case![Command::Rename].endpoint(rename))
                .branch(case![Command::RegGame].endpoint(reg_game))
                .branch(case![Command::Question].endpoint(question)),
        )
        .branch(
            case![State::RegistrationForTheGame]
                .branch(case![Command::Start].endpoint(start))
                .branch(case![Command::Help].endpoint(help))
                .branch(case![Command::Rules].endpoint(rules))
                .branch(case![Command::Playerdata].endpoint(player_data))
                .branch(case![Command::Statistics].endpoint(statistic))
                .branch(case![Command::Rename].endpoint(rename))
                .branch(case![Command::RegGame].endpoint(reg_game))
                .branch(case![Command::Question].endpoint(question)),
        );

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::ReceivePlayerRealFirstName].endpoint(receive_player_real_first_name)) //ввод игроком отчества
        .branch(
            case![State::ReceivePlayerRealPatronymic {
                player_real_first_name
            }]
            .endpoint(receive_player_real_patronymic),
        ) //ввод игроком фамилии
        .branch(
            case![State::ReceivePlayerRealLastName {
                player_real_first_name,
                player_real_patronymic
            }]
            .endpoint(receive_player_real_last_name),
        ) //ввод игроком города
        .branch(
            case![State::ReceivePlayerRealLocation {
                player_real_first_name,
                player_real_patronymic,
                player_real_last_name
            }]
            .endpoint(receive_player_real_location),
        ) //ввод игроком телефона
        .branch(
            case![State::ReceivePlayerPhoneNumber {
                player_real_first_name,
                player_real_patronymic,
                player_real_last_name,
                player_real_location
            }]
            .endpoint(receive_player_phone_number),
        ) //подтверждение юзером данных
        .branch(case![State::RegistrationComplete].endpoint(registration_complete))
        .branch(case![State::NumberQuestionFromPlayer].endpoint(question))
        .branch(case![State::ReceiveSingleQuestionFromPlayer].endpoint(receive_single_question))
        .branch(
            case![State::ReceiveSingleAnswerQuestionFromPlayer {
                player_single_question
            }]
            .endpoint(receive_single_answer_from_player),
        )
        .branch(
            case![State::ReceiveTopicMultiQuestionFromPlayer]
                .endpoint(receive_topic_multi_question_from_player),
        )
        .branch(
            case![State::ReceiveFirstQuestionFromPlayer {
                player_topic_multi_question
            }]
            .endpoint(receive_first_question_from_player),
        )
        .branch(
            case![State::ReceiveFirstAnswerQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question
            }]
            .endpoint(receive_first_answer_question_from_player),
        )
        .branch(
            case![State::ReceiveSecondQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question
            }]
            .endpoint(receive_second_question_from_player),
        )
        .branch(
            case![State::ReceiveSecondAnswerQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question
            }]
            .endpoint(receive_second_answer_question_from_player),
        )
        .branch(
            case![State::ReceiveThirdQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question
            }]
            .endpoint(receive_third_question_from_player),
        )
        .branch(
            case![State::ReceiveThirdAnswerQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question
            }]
            .endpoint(receive_third_answer_question_from_player),
        )
        .branch(
            case![State::ReceiveFourthQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question,
                player_third_answer_question
            }]
            .endpoint(receive_fourth_question_from_player),
        )
        .branch(
            case![State::ReceiveFourthAnswerQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question,
                player_third_answer_question,
                player_fourth_question
            }]
            .endpoint(receive_fourth_answer_question_from_player),
        )
        .branch(
            case![State::ReceiveFifthQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question,
                player_third_answer_question,
                player_fourth_question,
                player_fourth_answer_question
            }]
            .endpoint(receive_fifth_question_from_player),
        )
        .branch(
            case![State::ReceiveFifthAnswerQuestionFromPlayer {
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question,
                player_third_answer_question,
                player_fourth_question,
                player_fourth_answer_question,
                player_fifth_question
            }]
            .endpoint(receive_fifth_answer_question_from_player),
        )
        .branch(dptree::endpoint(invalid_state));

    let callback_query_handler = Update::filter_callback_query()
        .branch(case![State::ConfirmationOfRegistration].endpoint(confirmation_of_registration))
        .branch(
            case![State::ChoiceNumberQuestionFromPlayer]
                .endpoint(choice_number_question_from_player),
        )
        .branch(case![State::RegistrationForTheGame].endpoint(choice_registration_for_the_game));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        .branch(callback_query_handler)
}

//функция подключения игрока к боту
async fn start(bot: Bot, dialogue: MyDialogue, msg: Message, _: State) -> HandlerResult {
    //Получаем данные об игроке при подключении
    let player_id: Option<i64> = msg.from().map(|user| user.id.0 as i64); //user это структура библиотеки teloxide
    let player_name: Option<String> = msg.from().and_then(|user| user.username.clone());
    let player_first_name: Option<String> =
        msg.from().and_then(|user| Some(user.first_name.clone()));
    let player_last_name: Option<String> = msg.from().and_then(|user| user.last_name.clone());
    let chat_id: Option<ChatId> = msg.from().map(|chat| ChatId::from(chat.id));

    println!("подключился {:?}", player_id);

    let player_data = PlayerData {
        player_id: player_id.map(|id| id.into()),
        player_name: player_name.clone(),
        player_first_name: player_first_name.clone(),
        player_last_name: player_last_name.clone(),
        chat_id: chat_id.clone(),
    };

    // Проверяем наличие player_id в базе данных
    if let Some(player_id) = player_id {
        let player_profile = db::get_player_profile().await;

        match player_profile {
            Ok(player_profiles) => {
                // Проверяем, есть ли player_id в векторе игроков
                let player_id_exists = player_profiles
                    .iter()
                    .any(|player| player.player_id == Option::from(player_id));

                if !player_id_exists {
                    // Если player_id отсутствует, отправляем "знакомство"
                    bot.send_message(msg.chat.id, "🤝".to_string()).await?;
                    //отправляем второе сообщение
                    bot.send_message(
                        msg.chat.id,
                        "Здравствуйте!!!\nМы еще не знакомы. Давайте это исправим!!!".to_string(),
                    )
                    .await?;
                    bot.send_message(
                        msg.chat.id,
                        "Для этого необходимо выполнить несколько простых шагов.".to_string(),
                    )
                    .await?;

                    bot.send_message(
                        msg.chat.id,
                        "Напишите своё ИМЯ.\
        Ваше ИМЯ будет использоваться при обращении ведущего к Вам.",
                    )
                    .await?;

                    dialogue.update(State::ReceivePlayerRealFirstName).await?;
                } else {
                    // Если player_id существует, проверяем наличие player_real_first_name у этого игрока
                    let player_real_first_name_exists = player_profiles.iter().any(|player| {
                        player.player_id == Some(player_id)
                            && player.player_real_first_name.is_some()
                            && !player.player_real_first_name.clone().unwrap().is_empty()
                    });

                    if !player_real_first_name_exists {
                        // Если player_real_first_name отсутствует, выполняем нужные действия
                        bot.send_message(
                            msg.chat.id,
                            "Как хорошо, что Вы вернулись!\n\
                            Давайте продолжим знакомство."
                                .to_string(),
                        )
                        .await?;
                        bot.send_message(
                            msg.chat.id,
                            "Напишите своё ИМЯ.\
        Ваше ИМЯ будет использоваться при обращении ведущего к Вам.",
                        )
                        .await?;

                        dialogue.update(State::ReceivePlayerRealFirstName).await?;
                    } else {
                        bot.send_message(msg.chat.id, "Выберите команду в 'Меню'.")
                            .await?;
                        //если игрок зарегестирован переходим в состояние RegistrationComplete
                        dialogue.update(State::RegistrationComplete).await?;
                    }
                }
            }
            Err(err) => {
                // Обработка ошибок при получении данных об игроках из базы данных
                eprintln!("Ошибка при получении данных о пользователях: {}", err);
            }
        }
    }
    if let Err(err) = db::write_player_data_to_db(&player_data).await {
        eprintln!("Ошибка при записи в БД {:?}", err);
    }

    Ok(())
}

//получение имени игрока
async fn receive_player_real_first_name(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(text) => {
            if text.chars().any(|c| !c.is_alphabetic()) {
                bot.send_message(msg.chat.id, "Ваше имя должно содержать только буквы.")
                    .await?;
                bot.send_message(
                    msg.chat.id,
                    "Напишите своё ИМЯ.\
        Ваше ИМЯ будет использоваться при обращении ведущего к Вам.",
                )
                .await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Напишите своё ОТЧЕСТВО.\
                             Ваше ОТЧЕСТВО будет использоваться при обращении ведущего к Вам.",
                )
                .await?;
                dialogue
                    .update(State::ReceivePlayerRealPatronymic {
                        player_real_first_name: text.to_string(),
                    })
                    .await?;
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Отправляй только буквы.")
                .await?;
            bot.send_message(
                msg.chat.id,
                "Напишите своё ИМЯ.\
        Ваше ИМЯ будет использоваться при обращении ведущего к Вам.",
            )
            .await?;
        }
    }

    Ok(())
}

//получение отчества игрока
async fn receive_player_real_patronymic(
    bot: Bot,
    dialogue: MyDialogue,
    player_real_first_name: String, // Available from `State::ReceivePlayerRealFirstName`.
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_real_patronymic) => {
            if player_real_patronymic.chars().any(|c| !c.is_alphabetic()) {
                bot.send_message(msg.chat.id, "Ваше отчество должно содержать только буквы.")
                    .await?;
                bot.send_message(
                    msg.chat.id,
                    "Напишите своё ОТЧЕСТВО.\
                             Ваше ОТЧЕСТВО будет использоваться при обращении ведущего к Вам.",
                )
                .await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Напишите свою ФАМИЛИЮ.\
                             Ваша ФАМИЛИЯ будет использоваться при обращении ведущего к Вам.",
                )
                .await?;
                dialogue
                    .update(State::ReceivePlayerRealLastName {
                        player_real_first_name,
                        player_real_patronymic: player_real_patronymic.to_string(),
                    })
                    .await?;
            }
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляй только буквы.")
                .await?;
            bot.send_message(
                msg.chat.id,
                "Напишите своё ОТЧЕСТВО.\
                             Ваше ОТЧЕСТВО будет использоваться при обращении ведущего к Вам.",
            )
            .await?;
        }
    }

    Ok(())
}

//получение фамилии игрока
async fn receive_player_real_last_name(
    bot: Bot,
    dialogue: MyDialogue,
    (player_real_first_name, player_real_patronymic): (String, String), // Available from `State::ReceiveAge`.
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_real_last_name) => {
            if player_real_last_name.chars().any(|c| !c.is_alphabetic()) {
                bot.send_message(msg.chat.id, "Ваша фамилия должна содержать только буквы.")
                    .await?;
                bot.send_message(
                    msg.chat.id,
                    "Напишите свою ФАМИЛИЮ.\
                             Ваша ФАМИЛИЯ будет использоваться при обращении ведущего к Вам.",
                )
                .await?;
            } else {
                bot.send_message(msg.chat.id, "В каком ГОРОДЕ Вы играете? Необходимо указать официальное название Вашего города.")
                    .await?;
                dialogue
                    .update(State::ReceivePlayerRealLocation {
                        player_real_first_name,
                        player_real_patronymic: player_real_patronymic.to_string(),
                        player_real_last_name: player_real_last_name.to_string(),
                    })
                    .await?;
            }
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляй только буквы.")
                .await?;
            bot.send_message(
                msg.chat.id,
                "Напишите свою ФАМИЛИЮ.\
                             Ваша ФАМИЛИЯ будет использоваться при обращении ведущего к Вам.",
            )
            .await?;
        }
    }

    Ok(())
}

//получение города проживания игрока
async fn receive_player_real_location(
    bot: Bot,
    dialogue: MyDialogue,
    (player_real_first_name, player_real_patronymic, player_real_last_name): (
        String,
        String,
        String,
    ),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_real_location) => {
            if player_real_location.chars().any(|c| !c.is_alphabetic()) {
                bot.send_message(msg.chat.id, "Ваш город должен содержать только буквы")
                    .await?;
                bot.send_message(msg.chat.id, "В каком ГОРОДЕ Вы играете? Необходимо указать официальное название Вашего города.")
                    .await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    "Укажите Ваш НОМЕР ТЕЛЕФОНА в формате 89*********,\n\
            он необходим для связи организаторов с Вами в случае форс-мажора.\n\
            Если Вы не хотите указывать свой номер телефона, то введите любые 11 цифр, начиная с 1.",
                )
                    .await?;
                dialogue
                    .update(State::ReceivePlayerPhoneNumber {
                        player_real_first_name,
                        player_real_patronymic: player_real_patronymic.to_string(),
                        player_real_last_name: player_real_last_name.to_string(),
                        player_real_location: player_real_location.to_string(),
                    })
                    .await?;
            }
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляй только буквы.")
                .await?;
            bot.send_message(
                msg.chat.id,
                "В каком ГОРОДЕ Вы играете? Необходимо указать официальное название Вашего города.",
            )
            .await?;
        }
    }

    Ok(())
}

//получение номера телефона игрока
async fn receive_player_phone_number(
    //подтверждение юзером данных
    bot: Bot,
    dialogue: MyDialogue,
    (player_real_first_name, player_real_patronymic, player_real_last_name, player_real_location): (
        String,
        String,
        String,
        String,
    ),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_real_phone_number) => {
            // Проверяем, что строка не начинается с "+"
            if !player_real_phone_number.starts_with('+') {
                // Проверяем, что строка состоит из цифр и имеет длину 11 символов
                if player_real_phone_number.chars().all(|c| c.is_digit(10))
                    && player_real_phone_number.len() == 11
                {
                    // Попробуем преобразовать введенный текст в число
                    if let Ok(player_real_phone_number) = player_real_phone_number.parse::<i64>() {
                        let report = format!(
                            "Имя: {player_real_first_name}\n\
            Отчество: {player_real_patronymic}\n\
            Фамилия: {player_real_last_name}\n\
            Локация: {player_real_location}\n\
            Телефон: {player_real_phone_number}"
                        );

                        println!(
                            "Плэйер {} {} {} {} {}",
                            player_real_first_name,
                            player_real_patronymic,
                            player_real_last_name,
                            player_real_location,
                            player_real_phone_number
                        );

                        //создаём клавиатуру
                        let mut keyboard: Vec<Vec<InlineKeyboardButton>> = vec![];

                        let confirmation_of_registration_button = ["Подтверждаю", "Повторить"];

                        for versions in confirmation_of_registration_button.chunks(2) {
                            let row = versions
                                .iter()
                                .map(|&version| {
                                    InlineKeyboardButton::callback(
                                        version.to_owned(),
                                        version.to_owned(),
                                    )
                                })
                                .collect();

                            keyboard.push(row);
                        }

                        let confirmation_of_registration_keyboard =
                            InlineKeyboardMarkup::new(keyboard);

                        bot.send_message(msg.chat.id, report)
                            .reply_markup(confirmation_of_registration_keyboard)
                            .await?;

                        let player_id = msg.chat.id.0;

                        //запись регистрационных данных в промежуточную таблицу
                        let _ = rec_pre_reg_player(
                            player_id,
                            player_real_first_name,
                            player_real_patronymic,
                            player_real_last_name,
                            player_real_location,
                            player_real_phone_number,
                        )
                        .await;
                        println!("данные записаны в пром табл");

                        dialogue.update(State::ConfirmationOfRegistration).await?;
                    } else {
                        // Если преобразование не удалось, сообщаем игроку об ошибке
                        bot.send_message(
                            msg.chat.id,
                            "Введите только цифры в формате 89*********.",
                        )
                        .await?;
                    }
                } else {
                    // Если номер не состоит из цифр или имеет неправильную длину, сообщаем игроку об ошибке
                    bot.send_message(msg.chat.id, "Номер телефона должен состоять из 11 цифр.")
                        .await?;
                }
            } else {
                // Если строка начинается с "+", сообщаем игроку об ошибке
                bot.send_message(
                    msg.chat.id,
                    "Номер телефона не может начинаться с символа '+'.",
                )
                .await?;
            }
        }
        None => {
            bot.send_message(msg.chat.id, "Отправляй только цифры.")
                .await?;
        }
    }
    Ok(())
}

//подтверждение регистрационных данных игроком
async fn confirmation_of_registration(
    //колбэк юзером
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(version) = q.data {
        match version.as_str() {
            "Подтверждаю" => {
                let text = format!(
                    "Я очень рад нашему знакомству!\n\
                Нажмите /help для знакомства с моим функционалом."
                );
                bot.answer_callback_query(q.id).await?;
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, text).await?;

                    //получаем данные из пром табл и записываем в табл player
                    let player_id = chat.id.0;
                    let _ = rec_real_player_data_to_db(player_id).await;
                } else if let Some(id) = q.inline_message_id {
                    bot.edit_message_text_inline(id, text).await?;
                }

                dialogue.update(State::RegistrationComplete).await?;
            }

            "Повторить" => {
                let text = format!(
                    "😊 Давайте повторим.\n\
                    Для повторного ввода своих данных нажмите /rename"
                );
                bot.answer_callback_query(q.id).await?;
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, text).await?;
                } else if let Some(id) = q.inline_message_id {
                    bot.edit_message_text_inline(id, text).await?;
                }
                dialogue.update(State::RegistrationComplete).await?;
            }
            _ => {
                // Действие по умолчанию, если нажата неизвестная кнопка
            }
        }

        log::info!("You chose: {}", version);
    }

    Ok(())
}

//функция ответа на собщение игроку после регистрации
pub async fn registration_complete(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Выберите команду в 'Меню'")
        .await?;
    Ok(())
}

//отправка игроку сообщения об анонсе игры
pub async fn announce_game_bot(
    user_id: i64,
    game_day: String,
    game_time: String,
    game_location: String,
    price_player: i32,
    price_spectator: i32,
) -> HandlerResult {
    // Создаем соединение с базой данных
    let connection = establish_connection();

    // Получаем город организатора из таблицы users
    let user_city: Result<String, _> = connection
        .prepare("SELECT city FROM users WHERE id = ?")?
        .query_row(params![user_id], |row| row.get(0));

    // Если удалось получить город пользователя, продолжаем
    if let Ok(user_city) = user_city {
        // Получаем список player_id из таблицы players, у которых player_real_location совпадает с городом организатора
        let player_ids: Result<Vec<i64>, _> = connection
            .prepare("SELECT player_id FROM players WHERE player_real_location = ?")?
            .query_map(params![user_city], |row| row.get(0))
            .expect("Failed to get player_ids from the database")
            .collect();

        // Проходим по каждому player_id и отправляем сообщение
        for player_id in player_ids.unwrap_or_default() {
            // Преобразование player_id в ChatId
            let chat_id = ChatId(player_id);

            //создаем новый экземпляр бота
            let token = token::TELEGRAM_TOKEN;
            let bot = Bot::new(token);

            // Отправляем сообщение всем игрокам
            bot.send_message(
                chat_id,
                format!(
                    "Рад сообщить Вам, что объявлена регистрация на игру, которая пройдет {}.\n\
                    Начало игры в {}.\n\
                    Место проведения игры: {}.\n\
                    Стоимость участия в игре: {} руб.\n\
                    Стоимость присутсвия в зрительном зале: {} руб.\n\
                    Оплата наличными или банковским переводом.\n\
                    Для регистрации нажмите /reggame",
                    game_day, game_time, game_location, price_player, price_spectator
                ),
            )
            .await?;
        }
    }

    Ok(())
}

//отправка игроку сообщения об освобождении места в игре
pub async fn free_space_game_bot(game_id: i64) -> HandlerResult {
    // Создаем соединение с базой данных
    let connection = establish_connection();

    let mut stmt = connection.prepare(
        "SELECT game_day, game_time, game_location, price_player FROM register_games WHERE id = ?")
        .expect("не удалось выбрать даные из register_games в free_space_game_bot");
    let game_data = stmt
        .query_map(params![game_id], |row| {
            Ok((
                row.get::<usize, String>(0)?, //game_day
                row.get::<usize, String>(1)?, //game_time
                row.get::<usize, String>(2)?, //game_location
                row.get::<usize, i64>(3)?,    //price_player
            ))
        })
        .expect("не удалось выбрать данные из register_games в free_space_game_bot");

    // Проход по вектору и извлечение значений
    for result in game_data {
        if let Ok((game_day, game_time, game_location, price_player)) = result {
            // Получаем список reserve_player_id из таблицы reg_game_{}
            let reserve_player_ids: Result<Vec<i64>, _> = connection
                .prepare(&format!(
                    "SELECT reserve_player_id FROM reg_game_{} WHERE reserve_player_id IS NOT NULL",
                    game_id
                ))?
                .query_map(params![], |row| row.get(0))
                .expect(&format!(
                    "Failed to get reserve_player_ids from the reg_game_{}",
                    game_id
                ))
                .collect();

            // Проходим по каждому reserve_player_id и отправляем сообщение
            for reserve_player_id in reserve_player_ids.unwrap_or_default() {
                // Преобразование player_id в ChatId
                let chat_id = ChatId(reserve_player_id);

                //создаем новый экземпляр бота
                let token = token::TELEGRAM_TOKEN;
                let bot = Bot::new(token);

                // Отправляем сообщение всем игрокам
                bot.send_message(
                    chat_id,
                    format!(
                        "Рад сообщить Вам, что освободилось место на игру, которая пройдет {}.\n\
                    Начало игры в {}.\n\
                    Место проведения игры: {}.\n\
                    Стоимость участия в игре: {} руб.\n\
                    Оплата наличными или банковским переводом.\n\
                    Для регистрации нажмите /reggame",
                        game_day, game_time, game_location, price_player
                    ),
                )
                .await?;
            }
        }
    }

    Ok(())
}

//отправка сообщения об отмене игры
pub async fn del_game_bot(game_id: i64) -> HandlerResult {
    // Создаем соединение с базой данных
    let connection = establish_connection();

    let mut stmt = connection
        .prepare("SELECT game_day, game_time, game_location FROM register_games WHERE id = ?")
        .expect("не удалось выбрать даные из register_games в free_space_game_bot");
    let game_data = stmt
        .query_map(params![game_id], |row| {
            Ok((
                row.get::<usize, String>(0)?, //game_day
                row.get::<usize, String>(1)?, //game_time
            ))
        })
        .expect("не удалось выбрать данные из register_games в del_game_bot");

    // Проход по вектору и извлечение значений
    for result in game_data {
        if let Ok((game_day, game_time)) = result {
            // Получаем список всех зарегестрированных людей из таблицы reg_game_{}
            let all_ids: Result<Vec<i64>, _> = connection
                .prepare(&format!(
                    "SELECT player_id FROM reg_game_{} WHERE player_id IS NOT NULL
         UNION ALL
         SELECT reserve_player_id FROM reg_game_{} WHERE reserve_player_id IS NOT NULL
         UNION ALL
         SELECT spectator_id FROM reg_game_{} WHERE spectator_id IS NOT NULL",
                    game_id, game_id, game_id
                ))?
                .query_map(params![], |row| row.get(0))
                .expect("Ошибка при получении данных из reg_game_{} в del_game_bot")
                .collect();

            // Проходим по каждому player_id и отправляем сообщение
            for id in all_ids.unwrap_or_default() {
                // Преобразование player_id в ChatId
                let chat_id = ChatId(id);

                //создаем новый экземпляр бота
                let token = token::TELEGRAM_TOKEN;
                let bot = Bot::new(token);

                // Отправляем сообщение всем игрокам
                bot.send_message(
                    chat_id,
                    format!(
                        "Мне очень жаль, но организаторы вынуждены отменить игру, проведение которой планировалось {}.\n\
                    в {}.\n\
                    Предлагаю посмотреть список других объявленных игр /reggame",
                        game_day, game_time
                    ),
                ).await?;
            }
        }
    }

    Ok(())
}

//колбэк выбора регистрации на игру (игроком или зрителем)
async fn choice_registration_for_the_game(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(version) = q.data {
        // Разбиваем callback_data на части, чтобы получить тип и game_id
        let parts: Vec<&str> = version.split(':').collect();

        if parts.len() == 3 {
            let button_type = parts[0];
            let player_id: i64 = parts[1].parse().unwrap_or_default();
            let game_id: i64 = parts[2].parse().unwrap_or_default();

            //находим данные на игру на которую игрок регистрируется
            let connection = establish_connection();
            let game_data: Result<(String, String, String), _> = connection
                .prepare(
                    "SELECT game_day, game_time, game_location FROM register_games WHERE id = ?",
                )?
                .query_row(params![game_id], |row| {
                    Ok((
                        row.get(0)?, // game_day
                        row.get(1)?, // game_time
                        row.get(2)?, // game_location
                    ))
                });

            // Обработка результата запроса
            match game_data {
                Ok((game_day, game_time, game_location)) => {
                    //обработка выбранной кнопки
                    match button_type {
                        "player" => {
                            //обработка по результату проверки свободных мест в таблице game_{}
                            match reg_game_player(player_id, game_id) {
                                Ok(()) => {
                                    let text = format!(
                                        "Вы зарегистрированы на игру, \
                    которая состоится {} в {}.\nМесто проведения игры: {}.",
                                        game_day, game_time, game_location
                                    ); //сообщение которое увидит игрок в телеге
                                    bot.answer_callback_query(q.id).await?;
                                    if let Some(Message { id, chat, .. }) = q.message {
                                        bot.edit_message_text(chat.id, id, text).await?;

                                        println!("нажата кнопка у сообщения: {}", id);

                                        // Получаем message_id для удаления из чата
                                        let sent_messages = connection
                                            .prepare("SELECT message_id FROM message_id_del WHERE player_id = ?")?
                                            .query_map(params![player_id], |row| row.get::<usize, i32>(0))?
                                            .collect::<Result<Vec<_>, _>>()?;

                                        for message_id in sent_messages {
                                            // Преобразовываем i64 в MessageId
                                            let message_id_teloxide = MessageId(message_id);

                                            // Выводим в терминал значение каждого message_id
                                            println!(
                                                "Записанный в бд message_id: {}",
                                                message_id_teloxide
                                            );

                                            //удаляем сообщения кроме того, на котором была нажата кнопка
                                            if message_id_teloxide != id {
                                                bot.delete_message(chat.id, message_id_teloxide)
                                                    .await?;
                                            }
                                        }

                                        //удаляем все message_id из теблицы message_id_del
                                        connection.execute(
                                            "DELETE FROM message_id_del WHERE player_id = ?",
                                            params![player_id],
                                        ).expect("не удалось удалить данные из таблицы message_id_del");
                                    } else if let Some(id) = q.inline_message_id {
                                        bot.edit_message_text_inline(id, text).await?;
                                    }

                                    dialogue.update(State::RegistrationComplete).await?;
                                }
                                Err(()) => {
                                    let text = format!(
                                        "Мне очень жаль, но игровые места на игру, \
                    которая состоится {} в {} закончились. Вы зарегестрированы в резерв и в случае, если кто-то из \
                    огроков откажется от учатия в игре, я приглашу Вас к участию. Так же Вы можете зарегестрироваться на эту игру зрителем. \
                    Надеюсь увидеть Вас на наших играх.",
                                        game_day, game_time
                                    );
                                    bot.answer_callback_query(q.id).await?;
                                    if let Some(Message { id, chat, .. }) = q.message {
                                        bot.edit_message_text(chat.id, id, text).await?;

                                        // Получаем message_id для удаления из чата
                                        let sent_messages = connection
                                            .prepare("SELECT message_id FROM message_id_del WHERE player_id = ?")?
                                            .query_map(params![player_id], |row| row.get::<usize, i32>(0))?
                                            .collect::<Result<Vec<_>, _>>()?;

                                        for message_id in sent_messages {
                                            // Преобразовываем i64 в MessageId
                                            let message_id_teloxide = MessageId(message_id);

                                            // Выводим в терминал значение каждого message_id
                                            println!(
                                                "Записанный в бд message_id: {}",
                                                message_id_teloxide
                                            );

                                            //удаляем сообщения кроме того, на котором была нажата кнопка
                                            if message_id_teloxide != id {
                                                bot.delete_message(chat.id, message_id_teloxide)
                                                    .await?;
                                            }
                                        }

                                        //удаляем все message_id из теблицы message_id_del
                                        connection.execute(
                                            "DELETE FROM message_id_del WHERE player_id = ?",
                                            params![player_id],
                                        ).expect("не удалось удалить данные из таблицы message_id_del");
                                    } else if let Some(id) = q.inline_message_id {
                                        bot.edit_message_text_inline(id, text).await?;
                                    }

                                    dialogue.update(State::RegistrationComplete).await?;
                                }
                            }
                        }

                        "spectator" => {
                            //обработка по результату проверки свободных мест в таблице game_{}
                            match reg_game_spectator(player_id, game_id) {
                                Ok(()) => {
                                    let text = format!(
                                        "Мы ждем Вас в зрительном зале на игре, \
                    которая состоится {} в {}.\nМесто проведения игры: {}.",
                                        game_day, game_time, game_location
                                    );
                                    bot.answer_callback_query(q.id).await?;
                                    if let Some(Message { id, chat, .. }) = q.message {
                                        bot.edit_message_text(chat.id, id, text).await?;

                                        // Получаем message_id для удаления из чата
                                        let sent_messages = connection
                                            .prepare("SELECT message_id FROM message_id_del WHERE player_id = ?")?
                                            .query_map(params![player_id], |row| row.get::<usize, i32>(0))?
                                            .collect::<Result<Vec<_>, _>>()?;

                                        for message_id in sent_messages {
                                            // Преобразовываем i64 в MessageId
                                            let message_id_teloxide = MessageId(message_id);

                                            // Выводим в терминал значение каждого message_id
                                            println!(
                                                "Записанный в бд message_id: {}",
                                                message_id_teloxide
                                            );

                                            //удаляем сообщения кроме того, на котором была нажата кнопка
                                            if message_id_teloxide != id {
                                                bot.delete_message(chat.id, message_id_teloxide)
                                                    .await?;
                                            }
                                        }

                                        //удаляем все message_id из теблицы message_id_del
                                        connection.execute(
                                            "DELETE FROM message_id_del WHERE player_id = ?",
                                            params![player_id],
                                        ).expect("не удалось удалить данные из таблицы message_id_del");
                                    } else if let Some(id) = q.inline_message_id {
                                        bot.edit_message_text_inline(id, text).await?;
                                    }

                                    dialogue.update(State::RegistrationComplete).await?;
                                }

                                Err("Мест нет") => {
                                    let text = format!(
                                        "Мне очень жаль, но зрительный зал игры, \
которая состоится {} в {} полон. Надеюсь увидеть Вас на других играх.",
                                        game_day, game_time
                                    );
                                    bot.answer_callback_query(q.id).await?;
                                    if let Some(Message { id, chat, .. }) = q.message {
                                        bot.edit_message_text(chat.id, id, text).await?;

                                        // Получаем message_id для удаления из чата
                                        let sent_messages = connection
                                            .prepare("SELECT message_id FROM message_id_del WHERE player_id = ?")?
                                            .query_map(params![player_id], |row| row.get::<usize, i32>(0))?
                                            .collect::<Result<Vec<_>, _>>()?;

                                        for message_id in sent_messages {
                                            // Преобразовываем i64 в MessageId
                                            let message_id_teloxide = MessageId(message_id);

                                            // Выводим в терминал значение каждого message_id
                                            println!(
                                                "Записанный в бд message_id: {}",
                                                message_id_teloxide
                                            );

                                            //удаляем сообщения кроме того, на котором была нажата кнопка
                                            if message_id_teloxide != id {
                                                bot.delete_message(chat.id, message_id_teloxide)
                                                    .await?;
                                            }
                                        }

                                        //удаляем все message_id из теблицы message_id_del
                                        connection.execute(
                                            "DELETE FROM message_id_del WHERE player_id = ?",
                                            params![player_id],
                                        ).expect("не удалось удалить данные из таблицы message_id_del");
                                    } else if let Some(id) = q.inline_message_id {
                                        bot.edit_message_text_inline(id, text).await?;
                                    }

                                    dialogue.update(State::RegistrationComplete).await?;
                                }

                                Err("Зритель уже зарегистрирован") => {
                                    let text = format!(
                                        "Вы уже зарегистрированы зрителем на игру, \
                                        которая состоится {} в {}",
                                        game_day, game_time
                                    );
                                    bot.answer_callback_query(q.id).await?;
                                    if let Some(Message { id, chat, .. }) = q.message {
                                        bot.edit_message_text(chat.id, id, text).await?;

                                        // Получаем message_id для удаления из чата
                                        let sent_messages = connection
                                            .prepare("SELECT message_id FROM message_id_del WHERE player_id = ?")?
                                            .query_map(params![player_id], |row| row.get::<usize, i32>(0))?
                                            .collect::<Result<Vec<_>, _>>()?;

                                        for message_id in sent_messages {
                                            // Преобразовываем i64 в MessageId
                                            let message_id_teloxide = MessageId(message_id);

                                            // Выводим в терминал значение каждого message_id
                                            println!(
                                                "Записанный в бд message_id: {}",
                                                message_id_teloxide
                                            );

                                            //удаляем сообщения кроме того, на котором была нажата кнопка
                                            if message_id_teloxide != id {
                                                bot.delete_message(chat.id, message_id_teloxide)
                                                    .await?;
                                            }
                                        }

                                        //удаляем все message_id из теблицы message_id_del
                                        connection.execute(
                                            "DELETE FROM message_id_del WHERE player_id = ?",
                                            params![player_id],
                                        ).expect("не удалось удалить данные из таблицы message_id_del");
                                    } else if let Some(id) = q.inline_message_id {
                                        bot.edit_message_text_inline(id, text).await?;
                                    }

                                    dialogue.update(State::RegistrationComplete).await?;
                                }

                                Err(_) => {
                                    // Обработка всех остальных вариантов ошибок
                                    // Например, вывод сообщения об ошибке в лог или другие действия
                                }
                            }
                        }

                        "delete_player" => {
                            //удаление игрока из таблицы reg_game_{}
                            delete_game_player(game_id, player_id).await;

                            let text = format!(
                                "Очень жаль, что Вы отказались от игры, \
                    которая состоится {} в {}.\nМесто проведения игры: {}.",
                                game_day, game_time, game_location
                            ); //сообщение которое увидит игрок в телеге
                            bot.answer_callback_query(q.id).await?;
                            if let Some(Message { id, chat, .. }) = q.message {
                                bot.edit_message_text(chat.id, id, text).await?;

                                // Получаем message_id для удаления из чата
                                let sent_messages = connection
                                    .prepare(
                                        "SELECT message_id FROM message_id_del WHERE player_id = ?",
                                    )?
                                    .query_map(params![player_id], |row| row.get::<usize, i32>(0))?
                                    .collect::<Result<Vec<_>, _>>()?;

                                for message_id in sent_messages {
                                    // Преобразовываем i64 в MessageId
                                    let message_id_teloxide = MessageId(message_id);

                                    // Выводим в терминал значение каждого message_id
                                    println!("Записанный в бд message_id: {}", message_id_teloxide);

                                    //удаляем сообщения кроме того, на котором была нажата кнопка
                                    if message_id_teloxide != id {
                                        bot.delete_message(chat.id, message_id_teloxide).await?;
                                    }
                                }

                                //удаляем все message_id из теблицы message_id_del
                                connection
                                    .execute(
                                        "DELETE FROM message_id_del WHERE player_id = ?",
                                        params![player_id],
                                    )
                                    .expect("не удалось удалить данные из таблицы message_id_del");
                            } else if let Some(id) = q.inline_message_id {
                                bot.edit_message_text_inline(id, text).await?;
                            }

                            dialogue.update(State::RegistrationComplete).await?;
                        }

                        _ => {
                            // Действие по умолчанию, если нажата неизвестная кнопка
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error retrieving game data: {:?}", err);
                    // Дополнительная обработка ошибки, если необходимо
                }
            }

            log::info!("You chose: {}", version);
        }
    }

    Ok(())
}

//отправка сообщени для регистрации игрока на игру
pub async fn reg_game(bot: Bot, msg: Message, dialogue: MyDialogue) -> HandlerResult {
    let player_id = msg.chat.id.0; // Преобразование ChatId в i64

    let connection = establish_connection();

    // Необходимо найти игры которые объявлены в городе игрока. Для этого создаём запрос в котором
    // используем оператор JOIN для объединения таблиц register_games, users, и players. Условие объединения
    // основано на равенстве user_id из register_games и id из users, а также на равенстве city из users и
    // player_real_location из players. Далее мы фильтруем по player_id из players, который будет
    // msg.chat.id.0.
    let query_all_game_id = "
            SELECT rg.*, u.*
            FROM register_games rg
            JOIN users u ON rg.user_id = u.id
            JOIN players p ON u.city = p.player_real_location
            WHERE p.player_id = ? AND rg.stage = 0;
        "; //stage = 1 говорит о том, что игра сыграна

    //собираем все id игр из таблицы register_games в вектор
    let results_game_id: Result<Vec<_>, _> = connection
        .prepare(query_all_game_id)?
        .query_map(params![player_id], |row| {
            let game_id_registr: i64 = row.get(0)?;
            Ok(game_id_registr)
        })?
        .collect();

    // Проверяем, пуст ли вектор results_game_id
    match &results_game_id {
        Ok(vec) => {
            if vec.is_empty() {
                // Вектор пуст, отправляем сообщение об отсутствии анонсированных игр
                bot.send_message(
                    msg.chat.id,
                    "В данный момент отсутвуют анонсированные игры.\n\
                Как только будеть объявлена регистрация на игру, я сообщу Вам об этом.",
                )
                .await?;
            } else {
            }
        }

        Err(err) => {
            println!("В reg_game Вектор results_game_id: {}", err);
            return Ok(());
        }
    }

    // Проходим по каждому id игры
    for game_id_registr in &results_game_id.unwrap_or_default() {
        // Подставляем id игры в название таблицы и проверяем наличие player_id в этой таблице
        let query_check_player_id = format!(
            "SELECT player_id FROM reg_game_{} WHERE player_id = {}",
            game_id_registr, player_id
        );

        let result_check_player_id: Result<i64, _> = connection
            .prepare(&query_check_player_id)?
            .query_row(params![], |row| row.get(0));

        //Формирование и отправка сообщений с кнопками на объявленные игры на которые игрок не зарегестрирован
        if result_check_player_id.is_err() {
            let query_available_games = "
        SELECT id, game_day, game_time, game_location
        FROM register_games
        WHERE id = ?";

            let results: Result<Vec<_>, _> = connection
                .prepare(query_available_games)?
                .query_map(params![game_id_registr], |row| {
                    let game_id: i64 = row.get(0)?;
                    let game_day: String = row.get(1)?;
                    let game_time: String = row.get(2)?;
                    let game_location: String = row.get(3)?;

                    Ok((game_id, game_day, game_time, game_location))
                })?
                .collect();

            // Отправляем сообщение для каждой найденной записи в таблице register_games
            for result in results.unwrap_or_default() {
                let (game_id, game_day, game_time, game_location) = result;

                // Создаем уникальную клавиатуру для каждого сообщения
                let confirmation_of_registration_keyboard = create_keyboard(game_id, player_id);

                // Отправляем сообщение и добавляем его идентификатор в вектор
                let sent_message = bot
                    .send_message(
                        msg.chat.id,
                        format!(
                            "Открыта регистрация на игру, которая пройдёт: {}\n\
                            Время начала игры: {}\n\
                            Место проведения игры: {}",
                            game_day, game_time, game_location
                        ),
                    )
                    .reply_markup(confirmation_of_registration_keyboard)
                    .await?;

                let message_id = sent_message.id.0 as i64;

                //запись sent_message_id в бд, для последующего удаления сообщений в боте
                connection
                    .execute(
                        "INSERT INTO message_id_del (player_id, message_id) VALUES (?1, ?2)",
                        params![player_id, message_id],
                    )
                    .expect("не удалось вставить данные о сообщениях на удаление");
            }
            dialogue.update(State::RegistrationForTheGame).await?;
        } else {
            //Формирование и отправка информационных сообщений с кнопкой отказа от игры на объявленные игры на которые игрок зарегестрирован
            let query_not_available_games = "
        SELECT id, game_day, game_time, game_location
        FROM register_games
        WHERE id = ?";

            let results: Result<Vec<_>, _> = connection
                .prepare(query_not_available_games)?
                .query_map(params![game_id_registr], |row| {
                    let game_id: i64 = row.get(0)?;
                    let game_day: String = row.get(1)?;
                    let game_time: String = row.get(2)?;
                    let game_location: String = row.get(3)?;

                    Ok((game_id, game_day, game_time, game_location))
                })?
                .collect();

            // Отправляем сообщение для каждой найденной записи в таблице register_games
            for result in results.unwrap_or_default() {
                let (game_id, game_day, game_time, game_location) = result;

                // Создаем уникальную кнопку для каждого сообщения
                let abandoning_game = create_abandoning_button(game_id, player_id);

                // Отправляем сообщение и добавляем его идентификатор в вектор
                let _sent_message = bot
                    .send_message(
                        msg.chat.id,
                        format!(
                            "Вы зарегестрированы на игру, которая состоится: {}\nв: {}.\nМесто проведения игры: {}",
                            game_day, game_time, game_location
                        ),
                    )
                    .reply_markup(abandoning_game)
                    .await?;
            }
            dialogue.update(State::RegistrationForTheGame).await?;
        }
    }
    Ok(())
}

//создание кнопки для отказа от игры
fn create_abandoning_button(game_id: i64, player_id: i64) -> InlineKeyboardMarkup {
    // Создаем кнопку с уникальными значениями callback_data и text для каждой кнопки
    let callback_data_player = format!("delete_player:{}:{}", player_id, game_id);
    let button =
        InlineKeyboardButton::callback("Отказаться от игры".to_owned(), callback_data_player);

    // Создаем InlineKeyboardMarkup с единственной кнопкой
    InlineKeyboardMarkup::default().append_row(vec![button])
}

fn create_keyboard(game_id: i64, player_id: i64) -> ReplyMarkup {
    // Создаем клавиатуру с уникальными значениями callback_data и text для каждой кнопки
    let callback_data_player = format!("player:{}:{}", player_id, game_id);

    let callback_data_spectator = format!("spectator:{}:{}", player_id, game_id);

    let keyboard: Vec<Vec<InlineKeyboardButton>> = vec![vec![
        InlineKeyboardButton::callback("Игроком".to_owned(), callback_data_player),
        InlineKeyboardButton::callback("Зрителем".to_owned(), callback_data_spectator),
    ]];

    InlineKeyboardMarkup::new(keyboard).into()
}

async fn invalid_state(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Не удается обработать сообщение.\n\
     Нажмите /start. После этого в 'Меню' можно выбрать доступную команду.",
    )
    .await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn rules(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Свояк - это интеллектуально-развлекательно-познавательная игра с индивидуальным зачетом. В игре участвует 16 человек. Формат игры 'Round Robin' (круговая система) в котором каждый игрок сыграет с каждым. Игроки разбиваются (случайным образом) на четвёрки в соответствии с форматом игры. Каждая четверка игроков занимает игровые места в соответствии с указанием ведущего, который озвучивает какой игрок за кнопкой какого цвета будет играть. Для игры подготавливается пакет вопросов, содержащий 54 темы по 5 вопросов в каждой теме. В одной теме каждый вопрос имеет свою цену (10, 20, 30, 40, 50 баллов). Цена вопроса увеличивается вместе со сложностью вопроса, другими словами, вопрос за 10 баллов самый лёгкий, а за 50 - самый сложный. Игра проводится по следующей схеме: \n
1.Проводится три тура, по четыре раунда каждый. \n
2. Каждая четверка отвечает на вопросы четырех тем (в общем 20 вопрос за раунд). \n
3. За правильный ответ начисляется количество баллов, соответствующих цене вопроса. За неправильный ответ снимается количество баллов, соответствующее стоимости вопроса. Если игрок воздержался от ответа - баланс остаётся неизменным. \n
4. Всё баллы, набранные игроком в течении трех туров, суммируются и по результату, четыре игрока занимающие лидирующие позиции по сумме баллов переходят в финал. \n
5. В финале играется один раунд из шести тем (30 вопросов). Перед началом раунда, все баллы, набранные финалистами в отборочных турах обнуляются. По результату финального раунда определяется победитель игры.
",)
        .await?;
    Ok(())
}

async fn statistic(bot: Bot, msg: Message) -> HandlerResult {
    let player_id = msg.chat.id.0;

    let player_statistic = get_player_statistic(player_id).await.unwrap();

    bot.send_message(
        msg.chat.id,
        &format!(
            "Вы сыграли {} игр,\n\
            в которых правильно ответили на {} вопросов,\n\
            не правильно ответили на {} вопросов.\n\
            Вы выиграли {} игр.\n\
            Во всех играх Вы набрали {} баллов.",
            player_statistic.player_play_games,
            player_statistic.player_correct_answer,
            player_statistic.player_incorrect_answer,
            player_statistic.player_win_games,
            player_statistic.player_total_score
        ),
    )
    .await?;

    Ok(())
}

//получение данных об игроке по запросу игрока (о самом себе)
async fn player_data(bot: Bot, msg: Message) -> HandlerResult {
    //запрос данных о юзере из базы данных
    let player_id: Option<i64> = msg.from().map(|player| player.id.0 as i64); //получаем player_id из msg
    if let Some(player_id) = player_id {
        // Получить данные игрока из базы данных
        let player_profile = get_player_profile().await;

        match player_profile {
            Ok(player_profiles) => {
                // Проверяем, есть ли player_id в векторе пользователей
                if let Some(player_profile) = player_profiles
                    .iter()
                    .find(|player| player.player_id == Some(player_id))
                {
                    // Собираем данные игрока
                    let mut message = format!("Ваши данные:\n");
                    if let Some(player_real_first_name) = &player_profile.player_real_first_name {
                        message.push_str(&format!("Имя: {}\n", player_real_first_name));
                    }
                    if let Some(player_real_patronymic) = &player_profile.player_real_patronymic {
                        message.push_str(&format!("Отчество: {}\n", player_real_patronymic));
                    }
                    if let Some(player_real_last_name) = &player_profile.player_real_last_name {
                        message.push_str(&format!("Фамилия: {}\n", player_real_last_name));
                    }
                    if let Some(player_real_location) = &player_profile.player_real_location {
                        message.push_str(&format!("Локация: {}\n", player_real_location));
                    }
                    if let Some(player_real_phone_number) = &player_profile.player_real_phone_number
                    {
                        message
                            .push_str(&format!("Номер телефона: {}\n", player_real_phone_number));
                    }

                    // Отправляем данные игроку в чат
                    bot.send_message(msg.chat.id, message).await?;
                } else {
                    bot.send_message(msg.chat.id, "Ваши данные не найдены.")
                        .await?;
                }
            }
            Err(_) => {
                bot.send_message(
                    msg.chat.id,
                    "Произошла ошибка при получении данных пользователя.",
                )
                .await?;
            }
        }
    }

    Ok(())
}

//изменение своих регистрационных данных игроком
async fn rename(bot: Bot, msg: Message, dialogue: MyDialogue) -> HandlerResult {
    bot.send_message(
        msg.chat.id,
        "Спасибо за предоставление актуальных данных.\n\
                Если Вы передумаете - нажмите /start\n\
                Приступим!",
    )
    .await?;

    bot.send_message(
        msg.chat.id,
        "Напишите своё ИМЯ.\
        Ваше ИМЯ будет использоваться при обращении ведущего к Вам.",
    )
    .await?;
    dialogue.update(State::ReceivePlayerRealFirstName).await?;
    Ok(())
}

//отправка игроком вопроса для игры
async fn question(bot: Bot, msg: Message, dialogue: MyDialogue) -> HandlerResult {
    // Определение клавиши "Один вопрос"
    let single_question_button =
        InlineKeyboardButton::callback("Один вопрос".to_owned(), "Один вопрос".to_owned());

    // Определение клавиши "Пять вопросов одной темы"
    let five_questions_button = InlineKeyboardButton::callback(
        "Пять вопросов одной темы".to_owned(),
        "Пять вопросов одной темы".to_owned(),
    );

    // Создание вектора для клавиатуры с двумя кнопками в одной строке
    let keyboard: Vec<Vec<InlineKeyboardButton>> =
        vec![vec![single_question_button], vec![five_questions_button]];

    // Создание клавиатуры
    let request_confirmation_keyboard = InlineKeyboardMarkup::new(keyboard);

    // Отправка сообщения с клавиатурой
    bot.send_message(msg.chat.id, "Отправить вопрос для игры.")
        .reply_markup(request_confirmation_keyboard)
        .await?;

    dialogue
        .update(State::ChoiceNumberQuestionFromPlayer)
        .await?;
    Ok(())
}

//колбэк выбор количества вопросов
async fn choice_number_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    q: CallbackQuery,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if let Some(version) = q.data {
        match version.as_str() {
            "Один вопрос" => {
                let text = "Напишите свой вопрос.";
                bot.answer_callback_query(q.id).await?;
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, text).await?;
                } else if let Some(id) = q.inline_message_id {
                    bot.edit_message_text_inline(id, text).await?;
                }

                dialogue
                    .update(State::ReceiveSingleQuestionFromPlayer)
                    .await?;
            }

            "Пять вопросов одной темы" => {
                let text = "Напишите тему вопросов.";
                bot.answer_callback_query(q.id).await?;
                if let Some(Message { id, chat, .. }) = q.message {
                    bot.edit_message_text(chat.id, id, text).await?;
                } else if let Some(id) = q.inline_message_id {
                    bot.edit_message_text_inline(id, text).await?;
                }
                dialogue
                    .update(State::ReceiveTopicMultiQuestionFromPlayer)
                    .await?;
            }
            _ => {
                // Действие по умолчанию, если нажата неизвестная кнопка
            }
        }
        log::info!("You chose: {}", version);
    }
    Ok(())
}

async fn receive_single_question(bot: Bot, msg: Message, dialogue: MyDialogue) -> HandlerResult {
    //получение одиночного вопроса от игрока
    match msg.text() {
        Some(text) => {
            bot.send_message(msg.chat.id, "Напишите ответ на Ваш вопрос.")
                .await?;
            dialogue
                .update(State::ReceiveSingleAnswerQuestionFromPlayer {
                    player_single_question: text.to_string(),
                })
                .await?;
        }
        None => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }
    Ok(())
}

async fn receive_single_answer_from_player(
    //получение одиночного ответа от игрока
    bot: Bot,
    dialogue: MyDialogue,
    player_single_question: String,
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_single_answer_question) => {
            let player_id: Option<i64> = msg.from().map(|player| player.id.0 as i64);

            // Добавляем вопрос в базу данных
            add_to_single_question_db(
                player_id,
                player_single_question,
                player_single_answer_question,
            )
            .await;

            bot.send_message(
                msg.chat.id,
                "Ваш вопрос принят.\n\
После модерации он будет использован в одной из игр.\n\
Большое Вам спасибо!",
            )
            .await?;
            dialogue.update(State::RegistrationComplete).await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }
    Ok(())
}

async fn receive_topic_multi_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
) -> HandlerResult {
    //получение первого вопроса от юзера
    match msg.text() {
        Some(text) => {
            bot.send_message(msg.chat.id, "Напишите первый вопрос.")
                .await?;
            dialogue
                .update(State::ReceiveFirstQuestionFromPlayer {
                    player_topic_multi_question: text.to_string(),
                })
                .await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }

    Ok(())
}

async fn receive_first_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    msg: Message,
    player_topic_multi_question: String,
) -> HandlerResult {
    match msg.text() {
        Some(player_first_question) => {
            bot.send_message(msg.chat.id, "Напишите ответ на первый вопрос.")
                .await?;

            dialogue
                .update(State::ReceiveFirstAnswerQuestionFromPlayer {
                    player_topic_multi_question,
                    player_first_question: player_first_question.to_string(),
                })
                .await?;
        }

        _ => {
            bot.send_message(msg.chat.id, "Отправляте только текст.")
                .await?;
        }
    }

    Ok(())
}

async fn receive_first_answer_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    (player_topic_multi_question, player_first_question): (String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_first_answer_question) => {
            bot.send_message(msg.chat.id, "Напишите второй вопрос.")
                .await?;

            dialogue
                .update(State::ReceiveSecondQuestionFromPlayer {
                    player_topic_multi_question,
                    player_first_question: player_first_question.to_string(),
                    player_first_answer_question: player_first_answer_question.to_string(),
                })
                .await?;
        }

        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }

    Ok(())
}

async fn receive_second_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    (player_topic_multi_question, player_first_question, player_first_answer_question): (
        String,
        String,
        String,
    ),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_second_question) => {
            bot.send_message(msg.chat.id, "Напишите ответ на второй вопрос.")
                .await?;

            dialogue
                .update(State::ReceiveSecondAnswerQuestionFromPlayer {
                    player_topic_multi_question,
                    player_first_question: player_first_question.to_string(),
                    player_first_answer_question: player_first_answer_question.to_string(),
                    player_second_question: player_second_question.to_string(),
                })
                .await?;
        }

        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }

    Ok(())
}

async fn receive_second_answer_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    (
        player_topic_multi_question,
        player_first_question,
        player_first_answer_question,
        player_second_question,
    ): (String, String, String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_second_answer_question) => {
            bot.send_message(msg.chat.id, "Напишите третий вопрос.")
                .await?;

            dialogue
                .update(State::ReceiveThirdQuestionFromPlayer {
                    player_topic_multi_question,
                    player_first_question: player_first_question.to_string(),
                    player_first_answer_question: player_first_answer_question.to_string(),
                    player_second_question: player_second_question.to_string(),
                    player_second_answer_question: player_second_answer_question.to_string(),
                })
                .await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }
    Ok(())
}

async fn receive_third_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    (
        player_topic_multi_question,
        player_first_question,
        player_first_answer_question,
        player_second_question,
        player_second_answer_question,
    ): (String, String, String, String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_third_question) => {
            bot.send_message(msg.chat.id, "Напишите ответ на третий вопрос.")
                .await?;

            dialogue
                .update(State::ReceiveThirdAnswerQuestionFromPlayer {
                    player_topic_multi_question,
                    player_first_question: player_first_question.to_string(),
                    player_first_answer_question: player_first_answer_question.to_string(),
                    player_second_question: player_second_question.to_string(),
                    player_second_answer_question: player_second_answer_question.to_string(),
                    player_third_question: player_third_question.to_string(),
                })
                .await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }
    Ok(())
}

async fn receive_third_answer_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    (
        player_topic_multi_question,
        player_first_question,
        player_first_answer_question,
        player_second_question,
        player_second_answer_question,
        player_third_question,
    ): (String, String, String, String, String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_third_answer_question) => {
            bot.send_message(msg.chat.id, "Напишите четвёртый вопрос.")
                .await?;

            dialogue
                .update(State::ReceiveFourthQuestionFromPlayer {
                    player_topic_multi_question,
                    player_first_question: player_first_question.to_string(),
                    player_first_answer_question: player_first_answer_question.to_string(),
                    player_second_question: player_second_question.to_string(),
                    player_second_answer_question: player_second_answer_question.to_string(),
                    player_third_question: player_third_question.to_string(),
                    player_third_answer_question: player_third_answer_question.to_string(),
                })
                .await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }
    Ok(())
}

async fn receive_fourth_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    (
        player_topic_multi_question,
        player_first_question,
        player_first_answer_question,
        player_second_question,
        player_second_answer_question,
        player_third_question,
        player_third_answer_question,
    ): (String, String, String, String, String, String, String),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_fourth_question) => {
            bot.send_message(msg.chat.id, "Напишите ответ на четвёртый вопрос.")
                .await?;

            dialogue
                .update(State::ReceiveFourthAnswerQuestionFromPlayer {
                    player_topic_multi_question,
                    player_first_question: player_first_question.to_string(),
                    player_first_answer_question: player_first_answer_question.to_string(),
                    player_second_question: player_second_question.to_string(),
                    player_second_answer_question: player_second_answer_question.to_string(),
                    player_third_question: player_third_question.to_string(),
                    player_third_answer_question: player_third_answer_question.to_string(),
                    player_fourth_question: player_fourth_question.to_string(),
                })
                .await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }
    Ok(())
}

async fn receive_fourth_answer_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    (
        player_topic_multi_question,
        player_first_question,
        player_first_answer_question,
        player_second_question,
        player_second_answer_question,
        player_third_question,
        player_third_answer_question,
        player_fourth_question,
    ): (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_fourth_answer_question) => {
            bot.send_message(msg.chat.id, "Напишите пятый вопрос.")
                .await?;

            dialogue
                .update(State::ReceiveFifthQuestionFromPlayer {
                    player_topic_multi_question,
                    player_first_question: player_first_question.to_string(),
                    player_first_answer_question: player_first_answer_question.to_string(),
                    player_second_question: player_second_question.to_string(),
                    player_second_answer_question: player_second_answer_question.to_string(),
                    player_third_question: player_third_question.to_string(),
                    player_third_answer_question: player_third_answer_question.to_string(),
                    player_fourth_question: player_fourth_question.to_string(),
                    player_fourth_answer_question: player_fourth_answer_question.to_string(),
                })
                .await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст")
                .await?;
        }
    }
    Ok(())
}

async fn receive_fifth_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    (
        player_topic_multi_question,
        player_first_question,
        player_first_answer_question,
        player_second_question,
        player_second_answer_question,
        player_third_question,
        player_third_answer_question,
        player_fourth_question,
        player_fourth_answer_question,
    ): (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_fifth_question) => {
            bot.send_message(msg.chat.id, "Напишите ответ на пятый вопрос.")
                .await?;

            dialogue
                .update(State::ReceiveFifthAnswerQuestionFromPlayer {
                    player_topic_multi_question,
                    player_first_question: player_first_question.to_string(),
                    player_first_answer_question: player_first_answer_question.to_string(),
                    player_second_question: player_second_question.to_string(),
                    player_second_answer_question: player_second_answer_question.to_string(),
                    player_third_question: player_third_question.to_string(),
                    player_third_answer_question: player_third_answer_question.to_string(),
                    player_fourth_question: player_fourth_question.to_string(),
                    player_fourth_answer_question: player_fourth_answer_question.to_string(),
                    player_fifth_question: player_fifth_question.to_string(),
                })
                .await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }
    Ok(())
}

async fn receive_fifth_answer_question_from_player(
    bot: Bot,
    dialogue: MyDialogue,
    (
        player_topic_multi_question,
        player_first_question,
        player_first_answer_question,
        player_second_question,
        player_second_answer_question,
        player_third_question,
        player_third_answer_question,
        player_fourth_question,
        player_fourth_answer_question,
        player_fifth_question,
    ): (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    ),
    msg: Message,
) -> HandlerResult {
    match msg.text() {
        Some(player_fifth_answer_question) => {
            let player_id: Option<i64> = msg.from().map(|player| player.id.0 as i64);

            let player_fifth_answer_question = player_fifth_answer_question.to_string();

            // Добавляем вопрос в базу данных
            db::add_to_multi_question_db(
                player_id,
                player_topic_multi_question,
                player_first_question,
                player_first_answer_question,
                player_second_question,
                player_second_answer_question,
                player_third_question,
                player_third_answer_question,
                player_fourth_question,
                player_fourth_answer_question,
                player_fifth_question,
                player_fifth_answer_question,
            )
            .await;

            bot.send_message(
                msg.chat.id,
                "Ваши вопросы приняты.\n\
После модерации они будут использованы в одной из игр.\n\
Большое Вам спасибо!",
            )
            .await?;
            dialogue.update(State::RegistrationComplete).await?;
        }
        _ => {
            bot.send_message(msg.chat.id, "Отправляйте только текст.")
                .await?;
        }
    }
    Ok(())
}

//отправка результатов игры участникам
pub async fn sending_game_results(game_results: Vec<PlayerResultGame>) -> HandlerResult {
    println!("запуск sending_game_results");

    // Проходим по каждому player_id и отправляем сообщение
    for result in game_results {
        // Преобразование player_id в ChatId
        let chat_id = ChatId(result.player_id);

        // создаем новый экземпляр бота
        let token = token::TELEGRAM_TOKEN;
        let bot = Bot::new(token);

        // Отправляем сообщение игрокам, сыгравшим в игре
        bot.send_message(
            chat_id,
            format!(
                "Ваш результат прошедшей игры:\n\
                Правильные ответы: {}\n\
                Неправильные ответы: {}\n\
                Сумма баллов: {}",
                result.positive_count, result.negative_count, result.sum_score
            ),
        )
        .await?;
    }
    println!("окончание sending_game_results");
    Ok(())
}
