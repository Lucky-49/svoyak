use rocket::http::{CookieJar, Status};
extern crate rand;
use crate::db::{establish_connection, get_organiser_city};
use crate::web::users::get_user_id_from_cookies;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rusqlite::{params, Connection};

use rocket::response::stream::{Event, EventStream};
use rocket::tokio::select;
use rocket::tokio::sync::broadcast::{error::RecvError, Sender};
use rocket::Shutdown;
use rocket::State;
use teloxide::prelude::*;

#[derive(Serialize)]
pub struct QuestionsPacResponse {
    id: i64,
    package_name: String,
}

#[derive(Serialize)]
pub struct QuestionsPacsResponse {
    questionspacs: Vec<QuestionsPacResponse>,
}

// Функция (отображение на странице с пакетами) для запроса названий готовых пакетов из базы данных (stage=1) и не привязанных к играм (в register_games package_id !=1)
fn query_questions_pacs_done_not_game(conn: &Connection, user_id: i64) -> QuestionsPacsResponse {
    // Здесь выполните SQL-запрос, чтобы получить все данные о пакетах вопросов
    // и верните их в виде вектора структур QuestionsPacResponse
    let mut stmt = conn
        .prepare(
            //AND stage=1 - выбрать записи у которых в столбце stage стоит 1
            "SELECT id, package_name
                 FROM register_questions_pac
                 WHERE user_id = ? AND stage = 1
                 AND NOT EXISTS (
                                SELECT 1
                                FROM register_games rg
                                WHERE rg.user_id = ? AND rg.package_id = register_questions_pac.id
                                )
                 UNION
                 SELECT rp.id, rp.package_name
                 FROM register_questions_pac rp
                 JOIN data_transfers dt ON rp.id = dt.package_id
                 WHERE dt.receiver_user_id = ?
                 AND NOT EXISTS (
                                SELECT 1
                                FROM register_games rg
                                WHERE rg.user_id = ? AND rg.package_id = rp.id
                                )
                 ",
        )
        .expect("Failed to prepare query");
    /* подключиться к data_transfers и по user_id найти совпадения в столбце receiver_user_id.
    Потом по найденным package_id из таблицы data_transfers найти все совпадения в столбце id
    таблицы register_questions_pac и исключить все совпадения по user_id и package_id найденные в
    таблице games */

    let pacs_data_iter = stmt
        .query_map(params![user_id, user_id, user_id, user_id], |row| {
            Ok(QuestionsPacResponse {
                id: row.get(0)?,
                package_name: row.get(1)?,
            })
        })
        .expect("Failed to query questions_pac");

    QuestionsPacsResponse {
        questionspacs: pacs_data_iter.map(|pacs_data| pacs_data.unwrap()).collect(),
    }
}

#[get("/pacs_data_done_not_game")]
pub fn get_pacs_done_not_game(cookies: &CookieJar) -> Json<QuestionsPacsResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            let conn = establish_connection();
            let all_pacs_done = query_questions_pacs_done_not_game(&conn, user_id);
            Json(all_pacs_done)
        }
        Err(_) => Json(QuestionsPacsResponse {
            questionspacs: vec![],
        }), // Вернуть объект QuestionsPacsResponse с пустым массивом в случае ошибки
    }
}

// Функция (отображение на странице обмена пакетами) для запроса названий готовых пакетов из базы данных (stage=1)
fn query_all_questions_pacs_done(conn: &Connection, user_id: i64) -> QuestionsPacsResponse {
    // Здесь выполните SQL-запрос, чтобы получить все данные о пакетах вопросов
    // и верните их в виде вектора структур QuestionsPacResponse
    let mut stmt = conn
        .prepare(
            //AND stage=1 - выбрать записи у которых в столбце stage стоит 1
            "SELECT id, package_name
                 FROM register_questions_pac
                 WHERE user_id = ? AND stage = 1

                 UNION
                 SELECT rp.id, rp.package_name
                 FROM register_questions_pac rp
                 JOIN data_transfers dt ON rp.id = dt.package_id
                 WHERE dt.receiver_user_id = ? AND dt.right_transfer_other = 1
                 AND NOT EXISTS (
                                SELECT 1
                                FROM register_games rg
                                WHERE rg.user_id = ? AND rg.package_id = rp.id
                                )
                 ",
        )
        .expect("Failed to prepare query");
    /* подключиться к data_transfers и по user_id найти совпадения в столбце receiver_user_id.
    Потом по найденным package_id из таблицы data_transfers найти все совпадения в столбце id
    таблицы register_questions_pac и исключить все совпадения по user_id и package_id найденные в
    таблице games */

    let pacs_data_iter = stmt
        .query_map(params![user_id, user_id, user_id], |row| {
            Ok(QuestionsPacResponse {
                id: row.get(0)?,
                package_name: row.get(1)?,
            })
        })
        .expect("Failed to query questions_pac");

    QuestionsPacsResponse {
        questionspacs: pacs_data_iter.map(|pacs_data| pacs_data.unwrap()).collect(),
    }
}

//отображение "готовых" пакетов на странице
#[get("/all_pacs_data_done")]
pub fn get_all_pacs_done(cookies: &CookieJar) -> Json<QuestionsPacsResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            let conn = establish_connection();
            let all_pacs_done =
                crate::web::data_form::query_all_questions_pacs_done(&conn, user_id);
            Json(all_pacs_done)
        }
        Err(_) => Json(QuestionsPacsResponse {
            questionspacs: vec![],
        }), // Вернуть объект QuestionsPacsResponse с пустым массивом в случае ошибки
    }
}

#[get("/pacs_data_not_done")]
pub fn get_pacs_not_done(cookies: &CookieJar) -> Json<QuestionsPacsResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            let conn = establish_connection();
            let all_pacs_not_done = query_questions_pacs_not_done(&conn, user_id);
            Json(all_pacs_not_done)
        }
        Err(_) => Json(QuestionsPacsResponse {
            questionspacs: vec![],
        }), // Вернуть объект QuestionsPacsResponse с пустым массивом в случае ошибки
    }
}

// Функция для запроса названий не готовых пакетов из базы данных (stage!=0)
fn query_questions_pacs_not_done(conn: &Connection, user_id: i64) -> QuestionsPacsResponse {
    // Здесь выполните SQL-запрос, чтобы получить все данные о пакетах вопросов
    // и верните их в виде вектора структур QuestionsPacResponse
    let mut stmt = conn
        .prepare("SELECT id, package_name FROM register_questions_pac WHERE user_id = ? AND NOT (stage = 1 OR stage IS NULL)")
        .expect("Failed to prepare query");

    let pacs_data_iter = stmt
        .query_map(params![user_id], |row| {
            Ok(QuestionsPacResponse {
                id: row.get(0)?,
                package_name: row.get(1)?,
            })
        })
        .expect("Failed to query questions_pac");

    QuestionsPacsResponse {
        questionspacs: pacs_data_iter.map(|pacs_data| pacs_data.unwrap()).collect(),
    }
}

//структура для получения названий пакетов вопросов в контекстное меню на сайте
#[derive(Serialize)]
pub struct QuestionsPacsContextResponse {
    pacs_names: Vec<QuestionsPacContext>,
}

#[derive(Serialize)]
pub struct QuestionsPacContext {
    id: i64,
    name: String,
}

#[get("/pacs_data_context_menu")]
pub fn get_pacs_context_menu(cookies: &CookieJar) -> Json<QuestionsPacsContextResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            let conn = establish_connection();
            let pacs_names = query_questions_pacs_context_menu(&conn, user_id);
            Json(QuestionsPacsContextResponse { pacs_names })
        }
        Err(_) => Json(QuestionsPacsContextResponse { pacs_names: vec![] }), // Вернуть объект с пустым массивом в случае ошибки
    }
}

// Функция для запроса названий пакетов из базы данных (stage!=0) для контекстного меню
fn query_questions_pacs_context_menu(conn: &Connection, user_id: i64) -> Vec<QuestionsPacContext> {
    // Здесь выполните SQL-запрос, чтобы получить все данные о пакетах вопросов
    // и верните их в виде вектора структур QuestionsPacResponse
    let mut stmt = conn
        .prepare("SELECT id, package_name FROM register_questions_pac WHERE user_id = ? AND NOT (stage = 1 OR stage IS NULL)")
        .expect("Failed to prepare query");

    let pacs_names_iter = stmt
        .query_map(params![user_id], |row| {
            Ok(QuestionsPacContext {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        })
        .expect("Failed to query question pac names");

    pacs_names_iter
        .map(|pacs_names| pacs_names.unwrap())
        .collect()
}

//структура для получения тем пятёрок вопросов во втором контекстном меню на сайте
#[derive(Serialize)]
pub struct TopicsContextResponse {
    topics_names: Vec<TopicContext>,
}

#[derive(Serialize)]
pub struct TopicContext {
    name: String,
}

#[get("/topics_data_context_menu/<pac_id>")]
pub fn get_topics_context_menu(cookies: &CookieJar, pac_id: i64) -> Json<TopicsContextResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let conn = establish_connection();
            let topics_names = query_topic_context_menu(&conn, pac_id);
            Json(TopicsContextResponse { topics_names })
        }
        Err(_) => Json(TopicsContextResponse {
            topics_names: vec![],
        }), // Вернуть объект с пустым массивом в случае ошибки
    }
}

// Функция для запроса названий тем пятёрок из базы данных (stage!=0) для контекстного меню
fn query_topic_context_menu(conn: &Connection, pac_id: i64) -> Vec<TopicContext> {
    // Здесь выполните SQL-запрос, чтобы получить все данные о пятёрках вопросов
    // и верните их в виде вектора структур
    let sql_query = format!(
        "SELECT topic_five_questions FROM questions_pac_{} GROUP BY topic_five_questions HAVING COUNT(topic_five_questions) < 5", pac_id);

    let mut stmt = conn.prepare(&sql_query).expect("Failed to prepare query");

    let topics_names_iter = stmt
        .query_map(params![], |row| Ok(TopicContext { name: row.get(0)? }))
        .expect("Failed to query question pac names");

    topics_names_iter
        .map(|topics_names| topics_names.unwrap())
        .collect()
}

#[derive(Serialize, Debug)]
pub struct QuestionResponse {
    id: i64,
    topic_five_questions: String,
    question: String,
    answer: String,
    price_question: i32,
}

#[derive(Serialize, Debug)]
pub struct QuestionsResponse {
    questions: Vec<QuestionResponse>,
}

#[get("/questions_data/<questions_pac_id>")] //подготовка всех вопросов пакета для отображения на сайте
pub fn get_all_questions(cookies: &CookieJar, questions_pac_id: i64) -> Json<QuestionsResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let conn = establish_connection();
            let all_questions = query_questions(&conn, questions_pac_id);
            Json(all_questions)
        }
        Err(_) => Json(QuestionsResponse { questions: vec![] }), // Вернуть объект QuestionsResponse с пустым массивом в случае ошибки
    }
}

// Функция для запроса всех вопросов из пакета
fn query_questions(conn: &Connection, questions_pac_id: i64) -> QuestionsResponse {
    // Здесь выполните SQL-запрос, чтобы получить все данные о вопросах
    //проведя сортировку сначала по topic_five_questions, а потом по возрастанию цены вопроса
    // и верните их в виде вектора структур QuestionsResponse

    let sql_query = format!(
        "SELECT id, topic_five_questions, question, answer, price_question
        FROM questions_pac_{} ORDER BY topic_five_questions, price_question ASC",
        questions_pac_id
    );

    let mut stmt = conn.prepare(&sql_query).expect("Failed to prepare query");

    let questions_data_iter = stmt
        .query_map(params![], |row| {
            Ok(QuestionResponse {
                id: row.get(0)?,
                topic_five_questions: row.get(1)?,
                question: row.get(2)?,
                answer: row.get(3)?,
                price_question: row.get(4)?,
            })
        })
        .expect("Failed to query questions_pac");

    QuestionsResponse {
        questions: questions_data_iter
            .map(|questions_data| questions_data.unwrap())
            .collect(),
    }
}

#[get("/topic_questions_data/<questions_pac_id>/<topic_five_questions>")] //подготовка вопросов тематической пятёрки для отображения на сайте
pub fn get_topic_questions(
    cookies: &CookieJar,
    questions_pac_id: i64,
    topic_five_questions: String,
) -> Json<QuestionsResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let conn = establish_connection();
            let topic_questions =
                query_topic_questions(&conn, questions_pac_id, topic_five_questions);
            Json(topic_questions)
        }
        Err(_) => Json(QuestionsResponse { questions: vec![] }), // Вернуть объект QuestionsResponse с пустым массивом в случае ошибки
    }
}

// Функция для запроса вопросов темы
fn query_topic_questions(
    conn: &Connection,
    questions_pac_id: i64,
    topic_five_questions: String,
) -> QuestionsResponse {
    // Здесь выполните SQL-запрос, чтобы получить все данные о вопросах
    // и верните их в виде вектора структур QuestionsResponse

    let sql_query = format!(
        "SELECT id, topic_five_questions, question, answer, price_question FROM questions_pac_{} WHERE topic_five_questions = ?",
        questions_pac_id
    );

    let mut stmt = conn.prepare(&sql_query).expect("Failed to prepare query");

    let questions_data_iter = stmt
        .query_map(params![topic_five_questions], |row| {
            Ok(QuestionResponse {
                id: row.get(0)?,
                topic_five_questions: row.get(1)?,
                question: row.get(2)?,
                answer: row.get(3)?,
                price_question: row.get(4)?,
            })
        })
        .expect("Failed to query questions_pac");

    QuestionsResponse {
        questions: questions_data_iter
            .map(|questions_data| questions_data.unwrap())
            .collect(),
    }
}

#[derive(Serialize, Debug)]
pub struct TopicResponse {
    topic_five_questions: String,
}

#[derive(Serialize, Debug)]
pub struct TopicsResponse {
    topics: Vec<TopicResponse>,
}

#[get("/topics_data/<questions_pac_id>")] //подготовка тем вопросов пакета для отображения на сайте
pub fn get_all_topics(cookies: &CookieJar, questions_pac_id: i64) -> Json<TopicsResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let conn = establish_connection();
            let all_topics = query_topics(&conn, questions_pac_id);
            Json(all_topics)
        }
        Err(_) => Json(TopicsResponse { topics: vec![] }), // Вернуть объект с пустым массивом в случае ошибки
    }
}

// Функция для запроса всех тем вопросов из пакета
fn query_topics(conn: &Connection, questions_pac_id: i64) -> TopicsResponse {
    // Здесь выполните SQL-запрос, чтобы получить все данные о темах вопросов
    // и верните их в виде вектора структур

    let sql_query = format!(
        "SELECT topic_five_questions FROM questions_pac_{} GROUP BY topic_five_questions HAVING COUNT(topic_five_questions) < 5",
        questions_pac_id
    );

    let mut stmt = conn.prepare(&sql_query).expect("Failed to prepare query");

    let topics_data_iter = stmt
        .query_map(params![], |row| {
            Ok(TopicResponse {
                topic_five_questions: row.get(0)?,
            })
        })
        .expect("Failed to query questions_pac");

    TopicsResponse {
        topics: topics_data_iter
            .map(|topics_data| topics_data.unwrap())
            .collect(),
    }
}

#[get("/unique_topics_data/<questions_pac_id>")] //подготовка всех имеющихся тем вопросов пакета для отображения на сайте
pub fn get_all_topics_unique(cookies: &CookieJar, questions_pac_id: i64) -> Json<TopicsResponse> {
    println!("запуск unique_topics_data");

    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let conn = establish_connection();
            let all_topics = query_topics_unique(&conn, questions_pac_id);
            Json(all_topics)
        }
        Err(_) => Json(TopicsResponse { topics: vec![] }), // Вернуть объект с пустым массивом в случае ошибки
    }
}

// Функция для запроса всех тем вопросов из questions_pac
fn query_topics_unique(conn: &Connection, questions_pac_id: i64) -> TopicsResponse {
    // Здесь выполните SQL-запрос, чтобы получить все данные о темах вопросов
    // и верните их в виде вектора структур

    println!("запуск query_topics_unique");

    let sql_query = format!(
        "SELECT DISTINCT topic_five_questions FROM questions_pac_{}",
        questions_pac_id
    ); //выбираем уникальные названия тем пятёрок вопросов

    let mut stmt = conn.prepare(&sql_query).expect("Failed to prepare query");

    let topics_data_iter = stmt
        .query_map(params![], |row| {
            let topic = TopicResponse {
                topic_five_questions: row.get(0)?,
            };

            Ok(topic)
        })
        .expect("Failed to query questions_pac");

    TopicsResponse {
        topics: topics_data_iter
            .map(|topics_data| topics_data.unwrap())
            .collect(),
    }
}

// функция проверки общего количества вопросов в пакете
#[get("/questions_count/<questions_pac_id>")]
pub fn questions_count(questions_pac_id: i64) -> Json<i32> {
    let conn = establish_connection();

    // Проверяем количество записей в questions_pac_{}
    let count = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM questions_pac_{}", questions_pac_id),
            [],
            |row| row.get(0),
        )
        .unwrap_or_else(|err| {
            // Обработка ошибки
            eprintln!("Ошибка при проверке количества вопросов: {}", err);
            0 // Вернуть значение по умолчанию
        });

    // Отправить количество вопросов в качестве JSON-ответа
    Json(count)
}

//функция проверки состояния сервера (коннект/дисконнект)
#[get("/events")]
pub async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![] {
    let mut rx = queue.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(ref msg) => msg.clone(),
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}

#[derive(Serialize, Debug)]
pub struct PlayerQuestionResponse {
    player_real_last_name: String,
    player_real_first_name: String,
    player_real_patronymic: String,
    player_topic_five_questions: Option<String>,
    player_question: String,
    player_answer_question: String,
    id: i64,
    package_id: Option<i64>,
}

#[derive(Serialize, Debug)]
pub struct PlayersQuestionsResponse {
    questions: Vec<PlayerQuestionResponse>,
}
// Функция для запроса всех вопросов из таблицы questions_players (вопросы игроков)
#[get("/questions_players_data")]
pub fn get_all_questions_players(cookies: &CookieJar) -> Json<PlayersQuestionsResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            let conn = establish_connection();
            let all_questions_players = query_questions_players(&conn, user_id);
            Json(all_questions_players)
        }
        Err(_) => Json(PlayersQuestionsResponse { questions: vec![] }), // Вернуть объект QuestionsResponse с пустым массивом в случае ошибки
    }
}

// Функция для запроса всех вопросов из таблицы questions_players
fn query_questions_players(conn: &Connection, user_id: i64) -> PlayersQuestionsResponse {
    // Здесь выполните SQL-запрос, чтобы получить все данные о вопросах
    // и верните их в виде вектора структур QuestionsResponse
    // Выполнит SQL-запрос с использованием JOIN для объединения данных из обеих таблиц
    let sql_query = format!(
        "SELECT q.player_id, p.player_real_last_name, p.player_real_first_name, p.player_real_patronymic, q.player_topic_five_questions, q.player_question, q.player_answer_question, q.id, q.package_id
        FROM questions_players AS q
        INNER JOIN players AS p ON q.player_id = p.player_id
        INNER JOIN users AS u ON p.player_real_location = u.city
        WHERE u.id = {}", user_id
    );

    let mut stmt = conn.prepare(&sql_query).expect("Failed to prepare query");

    let questions_data_iter = stmt
        .query_map(params![], |row| {
            Ok(PlayerQuestionResponse {
                player_real_last_name: row.get(1)?,
                player_real_first_name: row.get(2)?,
                player_real_patronymic: row.get(3)?,
                player_topic_five_questions: row.get(4)?,
                player_question: row.get(5)?,
                player_answer_question: row.get(6)?,
                id: row.get(7)?,
                package_id: row.get(8)?,
            })
        })
        .expect("Failed to query questions_pac");

    PlayersQuestionsResponse {
        questions: questions_data_iter
            .map(|questions_data| questions_data.unwrap())
            .collect(),
    }
}

#[derive(Serialize, Debug)]
pub struct GameResponse {
    game_id: i64,
    game_day: String,
    game_time: String,
    game_location: String,
    questions_pac: Option<String>,
    players_count: i8,
    spectators_count: i8,
    questions_pac_id: Option<i64>,
}

#[derive(Serialize, Debug)]
pub struct GamesResponse {
    games: Vec<crate::web::data_form::GameResponse>,
}

#[get("/announce_games_data")] //подготовка всех объявленных игр для отображения на сайте
pub fn get_all_announce_games_data(cookies: &CookieJar) -> Json<GamesResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            let conn = establish_connection();
            let all_announce_games = query_announce_game(&conn, user_id);

            Json(all_announce_games)
        }
        Err(_) => Json(GamesResponse { games: vec![] }), // Вернуть объект с пустым массивом в случае ошибки
    }
}

// Функция для запроса всех анонсированных игр
fn query_announce_game(conn: &Connection, user_id: i64) -> GamesResponse {
    // Здесь выполните SQL-запрос, чтобы получить все данные по объявленным играм
    // и верните их в виде вектора структур
    //stage в register_games 0-игра объявлена, 1-игра сыграна, 2-игра отменена
    let mut stmt = conn //при выполнении sql-запроса преобразовываем дату для корректной работы в скрипте веб страницы
        .prepare("SELECT
                      r.id,
                      r.user_id,
                      SUBSTR(r.game_day, 7) || '-' || SUBSTR(r.game_day, 4, 2) || '-' || SUBSTR(r.game_day, 1, 2) as formatted_game_day,
                      r.game_time,
                      r.game_location,
                      r.package_id,
                      p.package_name

                   FROM register_games r
                   LEFT JOIN register_questions_pac p ON r.package_id = p.id
                   WHERE r.user_id = ? AND r.stage < 1
                   ORDER BY formatted_game_day, r.game_time")
        .expect("Ошибка при выборе анонсированных игр из базы данных query_announce_game");

    let games_data_iter = stmt
        .query_map(params![user_id], |row| {
            let game_id: i64 = row.get(0)?;

            // Извлекаем package_id из запроса
            let package_id: Option<i64> = row.get(5)?;

            // Выполняем запрос для получения данных из таблицы reg_game_{game_id}
            let game_table_name = format!("reg_game_{}", game_id);
            let game_query = format!(
                "SELECT
                              COUNT(player_id) as players_count,
                              COUNT(spectator_id) as spectators_count
                          FROM {}",
                game_table_name
            );

            // Выполняем запрос к таблице reg_game_{game_id} и считаем количество записей в столбцах player_id и spectator_id
            let count_result: Option<(i8, i8)> = conn
                .query_row(&game_query, params![], |row| {
                    Ok((row.get::<usize, i8>(0)?, row.get::<usize, i8>(1)?))
                })
                .ok();

            /*
            // Выводим в терминал количество игроков и зрителей
                        match count_result {
                            Some((players_count, spectators_count)) => {
                                println!("В игре {} зарегистрировано {} игроков и {} зрителей", game_id, players_count, spectators_count);
                            }
                            None => {
                                println!("Не удалось получить данные о количестве игроков и зрителей для игры {}", game_id);
                            }
                        }*/

            let game = GameResponse {
                game_id: row.get(0)?,
                game_day: row.get("formatted_game_day")?,
                game_time: row.get("game_time")?,
                game_location: row.get("game_location")?,
                questions_pac: row.get("package_name")?, // получаем название пакета из таблицы register_questions_pac
                players_count: count_result.map_or(0, |(players, _)| players),
                spectators_count: count_result.map_or(0, |(_, spectators)| spectators),
                questions_pac_id: package_id,
            };

            /*  println!("Game ID: {}", game.game_id);
            println!("Game Day: {}", game.game_day);
            println!("Game Time: {}", game.game_time);
            println!("Game Location: {}", game.game_location);
            println!("Questions Package: {:?}", game.questions_pac);
            println!("Players Count: {}", game.players_count);
            println!("Spectators Count: {}", game.spectators_count);
            println!("Questions Package ID: {:?}", game.questions_pac_id);
            println!("------------------------------------"); */

            Ok(game)
        })
        .expect("Failed to query query_announce_game");

    GamesResponse {
        games: games_data_iter
            .map(|games_data| games_data.unwrap())
            .collect(),
    }
}

//количество игроков зарегистрированных в городе
#[get("/players_count")]
pub fn players_count(cookies: &CookieJar) -> Result<Json<i32>, Status> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            println!("запуск players_count");

            let conn = establish_connection();

            //определяем город организатора
            let city = get_organiser_city(user_id);

            // Проверяем количество записей в таблице players
            let count: i32 = conn
                .query_row(
                    "SELECT COUNT(*) FROM players WHERE player_real_location = ?",
                    params![city],
                    |row| row.get(0),
                )
                .expect("Ошибка при проверке количества игроков зарегистрированных в городе");

            // Отправить количество вопросов в качестве JSON-ответа
            Ok(Json(count))
        }
        Err(_) => {
            // Пользователь не аутентифицирован
            Err(Status::Unauthorized)
        }
    }
}

//проверяем наличие таблицы schema_game_{}
pub fn table_exists_schema_questions(table_name: String) -> bool {
    println!("проверка наличия таблицы {}", table_name);
    //функция мониторит создание таблицы games
    let conn = establish_connection();

    conn.query_row(
        &format!(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = '{}'",
            table_name
        ),
        [],
        |row| row.get::<usize, i32>(0),
    )
    .is_ok()
}
#[get("/load_topic/<questions_pac_id>")]
pub async fn load_topic(cookies: &CookieJar<'_>, questions_pac_id: i64) -> Json<TopicsResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            let table_name = format!("schema_questions_{}_{}", user_id, questions_pac_id);

            let conn = establish_connection();

            if table_exists_schema_questions(table_name.clone()) {
                println!("Таблица {} найдена", table_name);
                // добавить query_topics_schema_questions
                let all_topics = query_topics_schema_questions(&conn, user_id, questions_pac_id);
                return Json(all_topics);
            } else {
                println!("Таблицы {} не существует", table_name);
                // Таблица не найдена, выполнить функцию get_all_topics_unique
                let all_topics = query_topics_unique(&conn, questions_pac_id);
                return Json(all_topics);
            }
        }
        _ => {}
    }
    // Возвращаем пустой объект, если произошла ошибка или таблица была найдена
    Json(TopicsResponse { topics: vec![] })
}

// Функция для запроса всех тем вопросов из schema_questions
fn query_topics_schema_questions(
    conn: &Connection,
    user_id: i64,
    questions_pac_id: i64,
) -> TopicsResponse {
    // Здесь выполните SQL-запрос, чтобы получить все данные о темах вопросов
    // и верните их в виде вектора структур

    println!("запуск query_topics_schema_questions");

    let sql_query = format!(
        "SELECT DISTINCT qp.topic_five_questions
        FROM schema_questions_{}_{} as qs
        JOIN questions_pac_{} as qp
        ON qs.question_id = qp.id",
        user_id, questions_pac_id, questions_pac_id
    );

    let mut stmt = conn.prepare(&sql_query).expect("Failed to prepare query");

    let topics_data_iter = stmt
        .query_map(params![], |row| {
            let topic = TopicResponse {
                topic_five_questions: row.get(0)?,
            };

            // println!("{:?}", topic); // Вывод в терминал

            Ok(topic)
        })
        .expect("Failed to query questions_pac");

    TopicsResponse {
        topics: topics_data_iter
            .map(|topics_data| topics_data.unwrap())
            .collect(),
    }
}
