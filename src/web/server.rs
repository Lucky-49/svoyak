use rocket::fs::{relative, FileServer}; /* Используется для настройки статического файлового
                                        сервера в Rocket. */
use crate::db::rec_schema_questions;
use crate::web::data_form::*; //подключение файла data_form
use crate::web::edit::*; //подключение файла edit
use crate::web::game::{
    cancellation_last_accrual_points, del_game, end_game, exclude_from_game, get_player_list,
    get_players_game_data, get_players_round_result, get_reserve_player_list, get_spectator_list,
    get_tour_result, player_list, players_dont_know_answer, rec_correct_answer_player,
    rec_incorrect_answer_player, start_game,
};
use crate::web::handlers::*;
use crate::web::rec_question::*;
use crate::web::transfer::{get_organiser_context_menu, package_transfer, transfer_question_pac};
use rocket::tokio::sync::broadcast::channel;
use rocket_dyn_templates::Template; //Импорт для работы с шаблонами в Rocket.
use teloxide::prelude::Message; //подключение файла rec_question

#[launch]
pub fn rocket() -> _ {
    rocket::build()
        .manage(channel::<Message>(1024).0)
        .mount(
            "/",
            routes![
                events,
                index,
                go_to_sign_up_page,
                go_to_log_in_page,
                go_to_home_organiser_page,
                players_count,
                register_user,
                login,
                prepare_questions_pac,
                logout,
                prepare_questions,
                prepare_game,
                rec_question_db,
                rec_question_db_2,
                re_rec_question_db,
                rec_questions_pac,
                create_questions_pac_context,
                create_topic_five_questions_context,
                rec_question_from_player_context,
                rec_in_topic_question_player,
                rec_topic_question_player_context,
                add_ques_in_topic,
                add_in_topic_question_player,
                get_pacs_done_not_game,
                get_all_pacs_done,
                get_pacs_not_done,
                get_all_questions,
                get_all_topics,
                get_all_topics_unique,
                get_all_questions_players,
                get_pacs_context_menu,
                get_topics_context_menu,
                get_topic_questions,
                get_organiser_context_menu,
                get_all_announce_games_data,
                get_players_game_data,
                get_players_round_result,
                get_tour_result,
                get_player_list,
                get_spectator_list,
                get_reserve_player_list,
                player_list,
                view_pac,
                edit_pac,
                edit_pac_que,
                questions_count,
                questions_players,
                del_player_question,
                exclude_from_game,
                add_topic,
                package_transfer,
                transfer_question_pac,
                announce_game,
                attach_questions_pac_to_game,
                navigate_prepare_game,
                start_game,
                rec_correct_answer_player,
                rec_incorrect_answer_player,
                cancellation_last_accrual_points,
                players_dont_know_answer,
                del_game,
                end_game,
                sequence_questions,
                load_topic,
                rec_schema_questions,
                random_topic,
            ],
        )
        .mount("/", FileServer::from(relative!("static"))) //указываем путь где лежат статические файлы для отображения html
        .attach(Template::fairing())
}
