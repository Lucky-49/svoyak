use crate::db::{
    del_reg_and_schema_game, delete_game_player, establish_connection, get_organiser_city,
    get_package_name, getting_game_results, update_statistic_players,
};
use crate::tg::bot::del_game_bot;
use crate::web::data_form::table_exists_schema_questions;
use crate::web::users::{get_user_id_from_cookies, get_user_role, Context};
use rand::prelude::SliceRandom;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket_dyn_templates::Template;
use rusqlite::{params, Connection, Error, OptionalExtension};
use tokio::task::spawn_blocking;

#[derive(Serialize)]
struct DataGamePage {
    header: String,
    game_id: i64,
    questions_pac_id: i64,
    pac_name: String,
    topic_five_questions: String,
    question_id: i32,
    last_name_player_question: String,
    first_name_player_question: String,
    patronymic_player_question: String,
    location_player_question: String,
    question: String,
    answer: String,
    price_question: i8,
    tour: i8,
    round: i8,
}
#[post("/start_game/<game_id>/<questions_pac_id>")]
pub async fn start_game(cookies: &CookieJar<'_>, game_id: i64, questions_pac_id: i64) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let conn = establish_connection();

                        let count_player_id: i64 = conn
                            .query_row(&format!("SELECT COUNT (player_id) FROM reg_game_{} WHERE player_id IS NOT NULL", game_id),
                                       params![], |row| row.get(0), )
                            .expect("ошибка подсчёта количества игроков в start_game");

                        if count_player_id == 16 {
                            let _ = round_group_players(cookies, game_id, questions_pac_id);

                            game(user_id, game_id, questions_pac_id)
                        } else {
                            let context = Context {
                                header: "Количество игроков менее 16-ти.".to_string(),
                            };
                            Template::render("404", &context)
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может проводить игры".to_string(),
                        };
                        Template::render("index", &context)
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваши права не определены".to_string(),
                    };
                    Template::render("index", &context)
                }
            }
        }
        Err(_) => {
            // Пользователь не аутентифицирован, перейдите на главную страницу
            let context = Context {
                header: "Стартовая страница".to_string(),
            };
            Template::render("index", &context)
        }
    }
}

fn game(user_id: i64, game_id: i64, questions_pac_id: i64) -> Template {
    let conn = establish_connection();

    // Находим максимальный id среди строк, где stage=1 (если не добавить эту проверку, то при обновлении страницы result_game 500: Internal Server Error)
    let max_id: Option<i32> = conn
        .query_row(
            &format!(
                "SELECT MAX(id) FROM schema_game_{} WHERE stage = 1",
                game_id
            ),
            [],
            |row| row.get(0),
        )
        .expect("не удалось выбрать max_id в changing_round");

    if max_id == Some(270) {
        result_tour(&conn, game_id, user_id, questions_pac_id, max_id)
    } else {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT q1.topic_five_questions
        FROM schema_game_{} AS s
        LEFT JOIN questions_pac_{} AS q1 ON s.question_id = q1.id
        WHERE s.stage IS NULL
        ORDER BY s.id ASC
        LIMIT 1",
                game_id, questions_pac_id
            ))
            .expect("название пятёрки вопросов не найдено");

        let topic_five_questions = stmt
            .query_row(params![], |row| row.get(0))
            .optional()
            .expect("название пятёрки вопросов не найдено");

        let mut stmt = conn
            .prepare(&format!(
                "SELECT q1.question
                                        FROM schema_game_{} AS s
        LEFT JOIN questions_pac_{} AS q1 ON s.question_id = q1.id
        WHERE s.stage IS NULL
        ORDER BY s.id ASC
        LIMIT 1",
                game_id, questions_pac_id
            ))
            .expect("Вопрос не найден");

        let question = stmt
            .query_row(params![], |row| row.get(0))
            .optional()
            .expect("Вопрос не найден");

        let mut stmt = conn
            .prepare(&format!(
                "SELECT q1.answer
                                        FROM schema_game_{} AS s
        LEFT JOIN questions_pac_{} AS q1 ON s.question_id = q1.id
        WHERE s.stage IS NULL
        ORDER BY s.id ASC
        LIMIT 1",
                game_id, questions_pac_id
            ))
            .expect("Ответ не найден");

        let answer = stmt
            .query_row(params![], |row| row.get(0))
            .optional()
            .expect("Ответ не найден");

        let mut stmt = conn
            .prepare(&format!(
                "SELECT q1.price_question
                                        FROM schema_game_{} AS s
        LEFT JOIN questions_pac_{} AS q1 ON s.question_id = q1.id
        WHERE s.stage IS NULL
        ORDER BY s.id ASC
        LIMIT 1",
                game_id, questions_pac_id
            ))
            .expect("Цена вопроса не найдена");

        let price_question = stmt
            .query_row(params![], |row| row.get(0))
            .expect("Цена вопроса не найдена");

        //находим данные игрока который задал вопрос (если этот вопрос принят организатором от игрока через телеграм)
        let mut stmt = conn
            .prepare(&format!(
                "SELECT q1.player_id
                                        FROM schema_game_{} AS s
        LEFT JOIN questions_pac_{} AS q1 ON s.question_id = q1.id
        WHERE s.stage IS NULL
        ORDER BY s.id ASC
        LIMIT 1",
                game_id, questions_pac_id
            ))
            .expect("Вопрос не найден");

        let player_id: Option<i64> = stmt
            .query_row(params![], |row| row.get(0))
            .optional()
            .unwrap_or(None); // Вместо использования expect, используем unwrap_or(None)

        println!("вопрос задал player_id {:?}", player_id);

        // Проверка наличия значения player_id
        let player_data_question = if let Some(player_id) = player_id {
            let mut stmt = conn
                .prepare("SELECT player_real_last_name, player_real_first_name, player_real_patronymic, player_real_location \
        FROM players WHERE player_id = ?")
                .expect("ошибка получения ФИО из players в game");

            Some(
                stmt.query_row(params![player_id], |row| {
                    Ok((
                        row.get::<usize, String>(0)?,
                        row.get::<usize, String>(1)?,
                        row.get::<usize, String>(2)?,
                        row.get::<usize, String>(3)?,
                    ))
                })
                .expect("ошибка формирования data_player в game"),
            )
        } else {
            None
        };

        let mut stmt = conn
            .prepare(&format!(
                "SELECT question_id
FROM schema_game_{}
WHERE stage IS NULL
LIMIT 1",
                game_id
            ))
            .expect("question_id вопроса не найден");

        let question_id = stmt
            .query_row(params![], |row| row.get(0))
            .expect("question_id вопроса не найден");

        let mut stmt = conn
            .prepare(&format!(
                "SELECT tour
FROM schema_game_{}
WHERE stage IS NULL
LIMIT 1",
                game_id
            ))
            .expect("question_id вопроса не найден");

        let tour = stmt
            .query_row(params![], |row| row.get(0))
            .expect("question_id вопроса не найден");

        let mut stmt = conn
            .prepare(&format!(
                "SELECT round
FROM schema_game_{}
WHERE stage IS NULL
LIMIT 1",
                game_id
            ))
            .expect("question_id вопроса не найден");

        let round = stmt
            .query_row(params![], |row| row.get(0))
            .expect("question_id вопроса не найден");

        let city = get_organiser_city(user_id);

        let package_name = get_package_name(questions_pac_id);

        let context = DataGamePage {
            header: city,
            game_id,
            questions_pac_id,
            pac_name: package_name,
            topic_five_questions: topic_five_questions
                .unwrap_or_else(|| "Название пятёрки вопросов не указано".to_string()),
            question_id,
            last_name_player_question: player_data_question
                .clone()
                .map(|data| data.0)
                .unwrap_or_else(|| "".to_string()),
            first_name_player_question: player_data_question
                .clone()
                .map(|data| data.1)
                .unwrap_or_else(|| "".to_string()),
            patronymic_player_question: player_data_question
                .clone()
                .map(|data| data.2)
                .unwrap_or_else(|| "".to_string()),
            location_player_question: player_data_question
                .clone()
                .map(|data| data.3)
                .unwrap_or_else(|| "".to_string()),
            question: question.unwrap_or_else(|| "Вопрос не указан".to_string()),
            answer: answer.unwrap_or_else(|| "Ответ не указан".to_string()),
            price_question,
            tour,
            round,
        };

        Template::render("game", &context)
    }
}

fn round_group_players(cookies: &CookieJar<'_>, game_id: i64, questions_pac_id: i64) {
    if !table_exists_schema_game(format!("schema_game_{}", game_id)) {
        let conn = establish_connection();

        // Создаем таблицы schema_game, если она не существует
        let _create_schema_game = conn
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS schema_game_{} (
                    id INTEGER PRIMARY KEY,
                    tour INTEGER,
                    round INTEGER,
                    questions_pac_id INTEGER,
                    question_id INTEGER,
                    player_id_1 INTEGER,
                    player_id_2 INTEGER,
                    player_id_3 INTEGER,
                    player_id_4 INTEGER,
                    stage INTEGER
                )",
                    game_id
                ),
                params![],
            )
            .expect("Ошибка создания таблицы");

        let mut stmt = conn
            .prepare(&format!(
                "SELECT player_id FROM reg_game_{} WHERE player_id IS NOT NULL",
                game_id
            ))
            .expect(&format!(
                "Ошибка при получении player_id из reg_game_{}",
                game_id
            ));

        let mut player_ids: Vec<i64> = stmt
            .query_map(params![], |row| Ok(row.get(0)?))
            .expect("Ошибка запроса в create_random_groups")
            .map(|result| result.unwrap())
            .collect();

        // Присваиваем случайные номера от 1 до 16
        player_ids.shuffle(&mut rand::thread_rng());

        // Вставляем записи в schema_game_{} с увеличением значения round на 1 при каждой итерации
        let mut round = 1; // Инициализируем round перед циклом

        for (_tour, skip) in [(1, 0), (2, 4), (2, 8), (2, 12)].iter().copied() {
            let insert_query = format!(
                "INSERT INTO schema_game_{} (tour, round, player_id_1, player_id_2, player_id_3, player_id_4) VALUES (?,?,?,?,?,?)",
                game_id
            );

            let players_for_insert: Vec<i64> =
                player_ids.iter().skip(skip).take(4).cloned().collect();

            for _ in 0..20 {
                conn.execute(
                    &insert_query,
                    params![
                        1,
                        round,
                        players_for_insert[0],
                        players_for_insert[1],
                        players_for_insert[2],
                        players_for_insert[3]
                    ],
                )
                .expect("Ошибка вставки данных в таблицу");
            }
            round += 1; // Увеличиваем round на 1 после каждой вставки
        }

        // Вставляем пятую запись с номерами 2, 6, 10, 14
        let insert_query = format!(
            "INSERT INTO schema_game_{} (tour, round, player_id_1, player_id_2, player_id_3, player_id_4) VALUES (?,?,?,?,?,?)",
            game_id
        );

        let players_for_insert: Vec<i64> = [1, 5, 9, 13]
            .iter()
            .map(|&index| player_ids[index as usize])
            .collect(); //собираем вектор из четырёх игроков

        for _ in 0..20 {
            conn.execute(
                &insert_query,
                params![
                    2,
                    1,
                    players_for_insert[0],
                    players_for_insert[1],
                    players_for_insert[2],
                    players_for_insert[3]
                ], //вставляем участников из содранного вектора
            )
            .expect("Ошибка вставки данных в таблицу");
        }

        // Вставляем пятую запись с номерами 1, 5, 9, 13
        let insert_query = format!(
            "INSERT INTO schema_game_{} (tour, round, player_id_1, player_id_2, player_id_3, player_id_4) VALUES (?,?,?,?,?,?)",
            game_id
        );

        let players_for_insert: Vec<i64> = [0, 4, 8, 12]
            .iter()
            .map(|&index| player_ids[index as usize])
            .collect(); //собираем вектор из четырёх игроков

        for _ in 0..20 {
            conn.execute(
                &insert_query,
                params![
                    2,
                    2,
                    players_for_insert[0],
                    players_for_insert[1],
                    players_for_insert[2],
                    players_for_insert[3]
                ], //вставляем участников из содранного вектора
            )
            .expect("Ошибка вставки данных в таблицу");
        }

        // Вставляем пятую запись с номерами 3, 7 11, 15
        let insert_query = format!(
            "INSERT INTO schema_game_{} (tour, round, player_id_1, player_id_2, player_id_3, player_id_4) VALUES (?,?,?,?,?,?)",
            game_id
        );

        let players_for_insert: Vec<i64> = [2, 6, 10, 14]
            .iter()
            .map(|&index| player_ids[index as usize])
            .collect(); //собираем вектор из четырёх игроков

        for _ in 0..20 {
            conn.execute(
                &insert_query,
                params![
                    2,
                    3,
                    players_for_insert[0],
                    players_for_insert[1],
                    players_for_insert[2],
                    players_for_insert[3]
                ], //вставляем участников из содранного вектора
            )
            .expect("Ошибка вставки данных в таблицу");
        }

        // Вставляем пятую запись с номерами 4, 8, 12, 16
        let insert_query = format!(
            "INSERT INTO schema_game_{} (tour, round, player_id_1, player_id_2, player_id_3, player_id_4) VALUES (?,?,?,?,?,?)",
            game_id
        );

        let players_for_insert: Vec<i64> = [3, 7, 11, 15]
            .iter()
            .map(|&index| player_ids[index as usize])
            .collect(); //собираем вектор из четырёх игроков

        for _ in 0..20 {
            conn.execute(
                &insert_query,
                params![
                    2,
                    4,
                    players_for_insert[0],
                    players_for_insert[1],
                    players_for_insert[2],
                    players_for_insert[3]
                ], //вставляем участников из содранного вектора
            )
            .expect("Ошибка вставки данных в таблицу");
        }

        // Вставляем пятую запись с номерами 1, 6, 11, 16
        let insert_query = format!(
            "INSERT INTO schema_game_{} (tour, round, player_id_1, player_id_2, player_id_3, player_id_4) VALUES (?,?,?,?,?,?)",
            game_id
        );

        let players_for_insert: Vec<i64> = [0, 5, 10, 15]
            .iter()
            .map(|&index| player_ids[index as usize])
            .collect(); //собираем вектор из четырёх игроков

        for _ in 0..20 {
            conn.execute(
                &insert_query,
                params![
                    3,
                    1,
                    players_for_insert[0],
                    players_for_insert[1],
                    players_for_insert[2],
                    players_for_insert[3]
                ], //вставляем участников из содранного вектора
            )
            .expect("Ошибка вставки данных в таблицу");
        }

        // Вставляем пятую запись с номерами 5, 10, 15, 4
        let insert_query = format!(
            "INSERT INTO schema_game_{} (tour, round, player_id_1, player_id_2, player_id_3, player_id_4) VALUES (?,?,?,?,?,?)",
            game_id
        );

        let players_for_insert: Vec<i64> = [4, 9, 14, 3]
            .iter()
            .map(|&index| player_ids[index as usize])
            .collect(); //собираем вектор из четырёх игроков

        for _ in 0..20 {
            conn.execute(
                &insert_query,
                params![
                    3,
                    2,
                    players_for_insert[0],
                    players_for_insert[1],
                    players_for_insert[2],
                    players_for_insert[3]
                ], //вставляем участников из содранного вектора
            )
            .expect("Ошибка вставки данных в таблицу");
        }

        // Вставляем пятую запись с номерами 9, 14, 3, 8
        let insert_query = format!(
            "INSERT INTO schema_game_{} (tour, round, player_id_1, player_id_2, player_id_3, player_id_4) VALUES (?,?,?,?,?,?)",
            game_id
        );

        let players_for_insert: Vec<i64> = [8, 13, 2, 7]
            .iter()
            .map(|&index| player_ids[index as usize])
            .collect(); //собираем вектор из четырёх игроков

        for _ in 0..20 {
            conn.execute(
                &insert_query,
                params![
                    3,
                    3,
                    players_for_insert[0],
                    players_for_insert[1],
                    players_for_insert[2],
                    players_for_insert[3]
                ], //вставляем участников из содранного вектора
            )
            .expect("Ошибка вставки данных в таблицу");
        }

        // Вставляем пятую запись с номерами 13, 2, 7, 12
        let insert_query = format!(
            "INSERT INTO schema_game_{} (tour, round, player_id_1, player_id_2, player_id_3, player_id_4) VALUES (?,?,?,?,?,?)",
            game_id
        );

        let players_for_insert: Vec<i64> = [12, 1, 6, 11]
            .iter()
            .map(|&index| player_ids[index as usize])
            .collect(); //собираем вектор из четырёх игроков

        for _ in 0..20 {
            conn.execute(
                &insert_query,
                params![
                    3,
                    4,
                    players_for_insert[0],
                    players_for_insert[1],
                    players_for_insert[2],
                    players_for_insert[3]
                ], //вставляем участников из содранного вектора
            )
            .expect("Ошибка вставки данных в таблицу");
        }

        // Вставляем пустые ячейки, в которые позже будут внесены финалисты игры
        let insert_query = format!(
            "INSERT INTO schema_game_{} (tour, round, player_id_1, player_id_2, player_id_3, player_id_4) VALUES (?,?,?,?,?,?)",
            game_id
        );

        for _ in 0..30 {
            conn.execute(
                &insert_query,
                params![4, 1, None::<i64>, None::<i64>, None::<i64>, None::<i64>],
            )
            .expect("Ошибка вставки данных в таблицу");
        }

        let _schema_sequence_questions = sequence_questions(cookies, questions_pac_id, game_id);
    } else {
    };

    if !table_exists_game(format!("game_{}", game_id)) {
        let conn = establish_connection();

        // Создаем таблицу результатов игры, если она не существует
        let _create_table_game = conn
            .execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS game_{} (
                    id INTEGER PRIMARY KEY,
                    questions_pac_id INTEGER,
                    tour INTEGER,
                    round INTEGER,
                    player_id INTEGER,
                    question_id INTEGER,
                    score INTEGER
                )",
                    game_id
                ),
                params![],
            )
            .expect("Ошибка создания таблицы");
    } else {
    }
}

//проверяем наличие таблицы schema_game_{}
fn table_exists_schema_game(table_name: String) -> bool {
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

//проверяем наличие таблицы game_{}
fn table_exists_game(table_name: String) -> bool {
    //функция мониторит создание таблицы game
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

fn sequence_questions(cookies: &CookieJar, questions_pac_id: i64, game_id: i64) {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            let conn = establish_connection();

            //Добавить проверку наличия schema_questions. Если таблица есть-переносим последовательность
            //в таблицу schema_game, если таблицы нет - выполняем случайный порядок.

            let table_name = format!("schema_questions_{}_{}", user_id, questions_pac_id);

            if table_exists_schema_questions(table_name.clone()) {
                let sql_query = format!(
                    "SELECT question_id FROM {} WHERE question_id IS NOT NULL",
                    table_name
                );

                let mut stmt = conn
                    .prepare(&sql_query)
                    .expect("ошибка подготовки запроса в sequence_questions");

                let questions_id_iter = stmt
                    .query_map(params![], |row| Ok(row.get::<usize, i64>(0)?))
                    .expect("ошибка получения question_id в sequence_questions");

                let questions_ids_vec: Vec<i64> = questions_id_iter
                    .map(|result| result.expect("Error reading question_id from database"))
                    .collect();

                // Используем UPDATE для обновления каждой строки в соответствии с элементами вектора
                let update_query = format!(
                    "UPDATE schema_game_{} SET questions_pac_id = {}, question_id = ?  WHERE id = ?",
                    game_id, questions_pac_id
                );

                // Используем подготовленный SQL-запрос для подготовки команды
                let mut command = conn
                    .prepare(&update_query)
                    .expect("Failed to prepare SQL command in sequence_questions");

                // Итерируем по каждому элементу вектора и выполняем обновление
                for (index, &question_id) in questions_ids_vec.iter().enumerate() {
                    let id = index + 1; // Предполагаем, что id строки начинается с 1
                    command
                        .execute(params![question_id, id])
                        .expect("Failed to execute SQL command");
                }

                //удаляем таблицу schema_questions_
                let _del_schema_questions = conn
                    .execute(
                        &format!(
                            "DROP TABLE IF EXISTS schema_questions_{}_{}",
                            user_id, questions_pac_id
                        ),
                        params![],
                    )
                    .expect(&format!(
                        "не удалось удалить таблицу schema_questions_{}_{}",
                        user_id, questions_pac_id
                    ));
            } else {
                let sql_query = format!(
                    "SELECT id, topic_five_questions, question, answer, price_question
        FROM questions_pac_{} ORDER BY topic_five_questions, price_question ASC",
                    questions_pac_id
                );

                let mut stmt = conn.prepare(&sql_query).expect("Failed to prepare query");
                let question_id_iter = stmt
                    .query_map(params![], |row| Ok(row.get::<usize, i64>(0)?))
                    .expect("Failed to query questions_pac");

                let mut rng = rand::thread_rng();

                let question_tuples: Vec<Vec<i64>> = question_id_iter
                    .collect::<Result<Vec<_>, _>>()
                    .expect("Ошибка в сборке вектора в sequence_questions")
                    .chunks(5)
                    .map(|chunk| chunk.to_vec())
                    .collect();

                let shuffled_question_tuples: Vec<Vec<i64>> = question_tuples
                    .choose_multiple(&mut rng, 54)
                    .cloned()
                    .collect();

                // Создаем пустой вектор для объединенных элементов
                let mut combined_question_tuple: Vec<i64> = Vec::new();

                // Идем по каждому кортежу в векторе
                for inner_tuple in shuffled_question_tuples {
                    // Добавляем все элементы текущего кортежа в общий вектор
                    combined_question_tuple.extend(inner_tuple);
                }

                // Используем UPDATE для обновления каждой строки в соответствии с элементами вектора
                let update_query = format!(
                    "UPDATE schema_game_{} SET questions_pac_id = ?, question_id = ?  WHERE id = ?",
                    game_id
                );

                // Используем подготовленный SQL-запрос для подготовки команды
                let mut command = conn
                    .prepare(&update_query)
                    .expect("Failed to prepare SQL command");

                // Итерируем по каждому элементу вектора и выполняем обновление
                for (index, &question_id) in combined_question_tuple.iter().enumerate() {
                    let id = index + 1; // Предполагаем, что id строки начинается с 1
                    command
                        .execute(params![questions_pac_id, question_id, id])
                        .expect("Failed to execute SQL command");
                }
            }
        }
        _ => {}
    }
}

#[derive(Serialize, Debug)]
struct PlayersGameData {
    id: i64,
    player_real_last_name: String,
    player_real_first_name: String,
    player_real_patronymic: String,
    game_id: i64,
    total_score: i64,
}

#[derive(Serialize, Debug)]
pub struct PlayersGameDataResponse {
    players: Vec<PlayersGameData>,
}

//отправка четвёрки участников (ФИО и id) на веб страницу
#[get("/get_players_game_data/<game_id>/<tour>/<round>")]
pub async fn get_players_game_data(
    cookies: &CookieJar<'_>,
    game_id: i64,
    tour: i8,
    round: i8,
) -> Json<PlayersGameDataResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let connection = establish_connection();
            let all_players_game = query_players_game(&connection, game_id, tour, round);

            Json(all_players_game)
        }
        Err(_) => Json(PlayersGameDataResponse { players: vec![] }), // Вернуть объект QuestionsResponse с пустым массивом в случае ошибки
    }
}

// Функция для запроса четверки игроков участвующих в раунде
fn query_players_game(
    conn: &Connection,
    game_id: i64,
    tour: i8,
    round: i8,
) -> PlayersGameDataResponse {
    // Запрос для player_id_1
    let sql_query_1 = format!(
        "SELECT
        g.player_id_1,
        p1.player_real_first_name as p1_first_name,
        p1.player_real_patronymic as p1_patronymic,
        p1.player_real_last_name as p1_last_name,
        (
            SELECT SUM(score)
            FROM game_{}
            WHERE player_id = g.player_id_1 AND tour = ? AND round = ?
        ) as total_score
    FROM schema_game_{} g
    LEFT JOIN players p1 ON g.player_id_1 = p1.player_id
    WHERE g.stage IS NULL
    ORDER BY g.id ASC
    LIMIT 1",
        game_id, game_id
    );

    let mut stmt_1 = conn
        .prepare(&sql_query_1)
        .expect("Failed to prepare query query_players_game");

    let players_data_iter_1 = stmt_1
        .query_map(params![tour, round], |row| {
            Ok(PlayersGameData {
                id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                game_id,
                total_score: row.get(4).unwrap_or(0), // Используйте unwrap_or, чтобы предоставить значение по умолчанию
            })
        })
        .expect("Failed to query query_players_game");

    // Запрос для player_id_2
    let sql_query_2 = format!(
        "SELECT
            g.player_id_2,
            p2.player_real_first_name as p2_first_name,
            p2.player_real_patronymic as p2_patronymic,
            p2.player_real_last_name as p2_last_name,
        (
            SELECT SUM(score)
            FROM game_{}
            WHERE player_id = g.player_id_2 AND tour = ? AND round = ?
        ) as total_score

        FROM schema_game_{} g
        LEFT JOIN players p2 ON g.player_id_2 = p2.player_id
        WHERE g.stage IS NULL
        ORDER BY g.id ASC
        LIMIT 1",
        game_id, game_id
    );

    let mut stmt_2 = conn
        .prepare(&sql_query_2)
        .expect("Failed to prepare query query_players_game");

    let players_data_iter_2 = stmt_2
        .query_map(params![tour, round], |row| {
            Ok(PlayersGameData {
                id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                game_id,
                total_score: row.get(4).unwrap_or(0), // Используйте unwrap_or, чтобы предоставить значение по умолчанию
            })
        })
        .expect("Failed to query query_players_game");

    // Запрос для player_id_3
    let sql_query_3 = format!(
        "SELECT
            g.player_id_3,
            p3.player_real_first_name as p3_first_name,
            p3.player_real_patronymic as p3_patronymic,
            p3.player_real_last_name as p3_last_name,
        (
            SELECT SUM(score)
            FROM game_{}
            WHERE player_id = g.player_id_3 AND tour = ? AND round = ?
        ) as total_score
        FROM schema_game_{} g
        LEFT JOIN players p3 ON g.player_id_3 = p3.player_id
        WHERE g.stage IS NULL
        ORDER BY g.id ASC
        LIMIT 1",
        game_id, game_id
    );

    let mut stmt_3 = conn
        .prepare(&sql_query_3)
        .expect("Failed to prepare query query_players_game");

    let players_data_iter_3 = stmt_3
        .query_map(params![tour, round], |row| {
            Ok(PlayersGameData {
                id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                game_id,
                total_score: row.get(4).unwrap_or(0), // Используйте unwrap_or, чтобы предоставить значение по умолчанию
            })
        })
        .expect("Failed to query query_players_game");

    // Запрос для player_id_4
    let sql_query_4 = format!(
        "SELECT
            g.player_id_4,
            p4.player_real_first_name as p4_first_name,
            p4.player_real_patronymic as p4_patronymic,
            p4.player_real_last_name as p4_last_name,
         (
            SELECT SUM(score)
            FROM game_{}
            WHERE player_id = g.player_id_4 AND tour = ? AND round = ?
        ) as total_score
        FROM schema_game_{} g
        LEFT JOIN players p4 ON g.player_id_4 = p4.player_id
        WHERE g.stage IS NULL
        ORDER BY g.id ASC
        LIMIT 1",
        game_id, game_id
    );

    let mut stmt_4 = conn
        .prepare(&sql_query_4)
        .expect("Failed to prepare query query_players_game");

    let players_data_iter_4 = stmt_4
        .query_map(params![tour, round], |row| {
            Ok(PlayersGameData {
                id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                game_id,
                total_score: row.get(4).unwrap_or(0), // Используйте unwrap_or, чтобы предоставить значение по умолчанию
            })
        })
        .expect("Failed to query query_players_game");

    // Собираем результаты
    let players_data_1: Vec<PlayersGameData> = players_data_iter_1
        .map(|players_data| players_data.unwrap())
        .collect();

    let players_data_2: Vec<PlayersGameData> = players_data_iter_2
        .map(|players_data| players_data.unwrap())
        .collect();

    let players_data_3: Vec<PlayersGameData> = players_data_iter_3
        .map(|players_data| players_data.unwrap())
        .collect();

    let players_data_4: Vec<PlayersGameData> = players_data_iter_4
        .map(|players_data| players_data.unwrap())
        .collect();

    // Объединяем результаты
    let mut players_data = players_data_1;
    players_data.extend(players_data_2);
    players_data.extend(players_data_3);
    players_data.extend(players_data_4);

    PlayersGameDataResponse {
        players: players_data,
    }
}

//обработка результата положительного ответа игрока
#[post("/rec_correct_answer_player/<player_id>/<game_id>/<questions_pac_id>/<question_id>/<price_question>/<tour>/<round>")]
pub async fn rec_correct_answer_player(
    cookies: &CookieJar<'_>,
    player_id: i64,
    game_id: i64,
    questions_pac_id: i64,
    question_id: i64,
    price_question: i32,
    tour: i8,
    round: i8,
) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => {
                    match role.as_str() {
                        "organiser" => {
                            let conn = establish_connection();

                            //проверка уникальности ответа игрока (необходимо для исключения повторного начисления баллов при обновлении страницы браузера)
                            if let Err(e) = result_answer_unique(
                                &conn,
                                game_id,
                                questions_pac_id,
                                tour,
                                round,
                                player_id,
                                question_id,
                                price_question,
                            ) {
                                eprintln!("Ошибка при проверке уникальности ответа: {}", e);
                                return changing_four_players(
                                    &conn,
                                    user_id,
                                    game_id,
                                    questions_pac_id,
                                );
                            }

                            let mut stmt = conn
                            .prepare(&format!("SELECT player_id_1 FROM schema_game_{} WHERE stage IS NULL ORDER BY id ASC
        LIMIT 1", game_id))
                            .expect("не удалось выбрать город организатора");

                            let player_id_1: i64 = stmt
                                .query_row(params![], |row| row.get(0))
                                .expect("город организатора не найден");

                            let mut stmt = conn
                            .prepare(&format!("SELECT player_id_2 FROM schema_game_{} WHERE stage IS NULL ORDER BY id ASC
        LIMIT 1", game_id))
                            .expect("не удалось выбрать город организатора");

                            let player_id_2: i64 = stmt
                                .query_row(params![], |row| row.get(0))
                                .expect("город организатора не найден");

                            let mut stmt = conn
                            .prepare(&format!("SELECT player_id_3 FROM schema_game_{} WHERE stage IS NULL ORDER BY id ASC
        LIMIT 1", game_id))
                            .expect("не удалось выбрать город организатора");

                            let player_id_3: i64 = stmt
                                .query_row(params![], |row| row.get(0))
                                .expect("город организатора не найден");

                            let mut stmt = conn
                            .prepare(&format!("SELECT player_id_4 FROM schema_game_{} WHERE stage IS NULL ORDER BY id ASC
        LIMIT 1", game_id))
                            .expect("не удалось выбрать город организатора");

                            let player_id_4: i64 = stmt
                                .query_row(params![], |row| row.get(0))
                                .expect("город организатора не найден");

                            // Создание вектора с идентификаторами игроков
                            let player_ids =
                                vec![player_id_1, player_id_2, player_id_3, player_id_4];

                            // Проверка совпадения среди всех игроков
                            if player_ids.contains(&player_id) {
                                for id in player_ids {
                                    let score = if id == player_id { price_question } else { 0 };
                                    conn.execute(
                                        &format!(
                                            "INSERT INTO game_{} (questions_pac_id, tour, round,
                                    player_id, question_id, score) VALUES (?, ?, ?, ?, ?, ?)",
                                            game_id
                                        ),
                                        params![
                                            questions_pac_id,
                                            tour,
                                            round,
                                            id,
                                            question_id,
                                            score
                                        ],
                                    )
                                    .expect("Failed to insert player into the database");
                                }
                            } else {
                                // println!("Player not found: {}", player_id);
                                // Дополнительная логика, если игрок не найден
                            }

                            update_stage_schema_game(game_id, question_id);

                            changing_four_players(&conn, user_id, game_id, questions_pac_id)
                        }
                        _ => {
                            // Пользователь не аутентифицирован, перейдите на главную страницу
                            let context = Context {
                                header: "Только организатор может проводить игры".to_string(),
                            };
                            Template::render("index", &context)
                        }
                    }
                }
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваши права не определены".to_string(),
                    };
                    Template::render("index", &context)
                }
            }
        }
        Err(_) => {
            // Пользователь не аутентифицирован, перейдите на главную страницу
            let context = Context {
                header: "Стартовая страница".to_string(),
            };
            Template::render("index", &context)
        }
    }
}

//проверка уникальности ответа игрока (необходимо для исключения повторного начисления баллов при обновлении страницы браузера)
fn result_answer_unique(
    conn: &Connection,
    game_id: i64,
    questions_pac_id: i64,
    tour: i8,
    round: i8,
    player_id: i64,
    question_id: i64,
    price_question: i32,
) -> Result<(), Error> {
    let abs_price_question = price_question.abs(); //необходимо вести сравнение по модулю числа иначе при проверке уникальности первого вопроса игры возникает ошибка с которой я не знаю как бороться

    let is_result_unique: bool = conn
        .query_row(&format!("SELECT COUNT (*)
    FROM game_{}
    WHERE questions_pac_id = ? AND tour = ? AND round = ? AND player_id =? AND question_id = ? AND ABS(score) = ?", game_id),
                   params![questions_pac_id, tour, round, player_id, question_id, abs_price_question],
                   |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
        )
        .expect(&format!("Не удалось выполнить проверку уникальности записи ответа игрока в таблицу game_{}", game_id));

    //если запись с такими параметрами уже существует-прекращаем выполнение
    if !is_result_unique {
        return Err(Error::QueryReturnedNoRows);
    }

    Ok(())
}

//обновляем stage вопроса в таблице schema_game (stage=1 означает, что вопрос сыгран)
fn update_stage_schema_game(game_id: i64, question_id: i64) {
    let conn = establish_connection();

    conn.execute(
        &format!(
            "UPDATE schema_game_{} SET stage = 1 WHERE question_id = ?",
            game_id
        ),
        params![question_id],
    )
    .expect(&format!(
        "Вставка stage в таблицу schema_game_{} не удалась",
        game_id
    ));

    formation_finalists_players(game_id);
}

//обработка результата отрицательного ответа игрока
#[post("/rec_incorrect_answer_player/<player_id>/<game_id>/<questions_pac_id>/<question_id>/<price_question>/<tour>/<round>")]
pub async fn rec_incorrect_answer_player(
    cookies: &CookieJar<'_>,
    player_id: i64,
    game_id: i64,
    questions_pac_id: i64,
    question_id: i64,
    price_question: i32,
    tour: i8,
    round: i8,
) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let price_question = price_question * -1; //одинаковое название до и после умножения обусловлено необходимостью передачи в result_answer_unique

                        let conn = establish_connection();

                        //проверка уникальности ответа игрока (необходимо для исключения повторного начисления баллов при обновлении страницы браузера)
                        if let Err(_) = result_answer_unique(
                            &conn,
                            game_id,
                            questions_pac_id,
                            tour,
                            round,
                            player_id,
                            question_id,
                            price_question,
                        ) {
                            // eprintln!("Ошибка при проверке уникальности ответа: {}", e);
                            return game(user_id, game_id, questions_pac_id);
                        }

                        conn.execute(
                            &format!("INSERT INTO game_{} (questions_pac_id, tour, round, player_id, question_id, score) VALUES (?, ?, ?, ?, ?, ?)", game_id),
                            params![questions_pac_id, tour, round, player_id, question_id, price_question],
                        ).expect("Failed to insert player into the database");

                        //вариант, если все четверо игроков ответили не верно - смена вопроса с отметкой в stage
                        let count: i8 = conn
                            .query_row(
                                &format!(
                                    "SELECT COUNT(*) AS count_score
         FROM (
            SELECT score
            FROM game_{}
            ORDER BY id DESC
            LIMIT 4
         ) AS subquery
         GROUP BY score",
                                    game_id
                                ),
                                params![],
                                |row| row.get(0),
                            )
                            .expect(
                                "ошибка подсчёта количества одинаковых неправильных
                        ответов на один вопрос в rec_incorrect_answer_player",
                            );

                        if count == 4 {
                            update_stage_schema_game(game_id, question_id);

                            changing_four_players(&conn, user_id, game_id, questions_pac_id)
                        } else {
                            game(user_id, game_id, questions_pac_id)
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может проводить игры".to_string(),
                        };
                        Template::render("index", &context)
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваши права не определены".to_string(),
                    };
                    Template::render("index", &context)
                }
            }
        }
        Err(_) => {
            // Пользователь не аутентифицирован, перейдите на главную страницу
            let context = Context {
                header: "Стартовая страница".to_string(),
            };
            Template::render("index", &context)
        }
    }
}

#[post("/players_dont_know_answer/<game_id>/<questions_pac_id>/<question_id>/<tour>/<round>")]
pub async fn players_dont_know_answer(
    cookies: &CookieJar<'_>,
    game_id: i64,
    questions_pac_id: i64,
    question_id: i64,
    tour: i8,
    round: i8,
) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let conn = establish_connection();

                        // Находим максимальный id среди строк, где stage=1 (если не добавить эту проверку, то при обновлении страницы result_game 500: Internal Server Error)
                        let max_id: Option<i32> = conn
                            .query_row(
                                &format!(
                                    "SELECT MAX(id) FROM schema_game_{} WHERE stage = 1",
                                    game_id
                                ),
                                [],
                                |row| row.get(0),
                            )
                            .expect("не удалось выбрать max_id в changing_round");

                        if max_id == Some(270) {
                            result_tour(&conn, game_id, user_id, questions_pac_id, max_id)
                        } else {
                            let mut stmt = conn
                                .prepare(&format!("SELECT player_id_1 FROM schema_game_{} WHERE stage IS NULL ORDER BY id ASC
        LIMIT 1", game_id))
                                .expect(&format!("player_id_1 в таблице schema_game_{} не найден", game_id));

                            let player_id_1: i64 =
                                stmt.query_row(params![], |row| row.get(0)).expect(&format!(
                                    "player_id_1 в таблице schema_game_{} не найден",
                                    game_id
                                ));

                            let mut stmt = conn
                                .prepare(&format!("SELECT player_id_2 FROM schema_game_{} WHERE stage IS NULL ORDER BY id ASC
        LIMIT 1", game_id))
                                .expect("не удалось выбрать город организатора");

                            let player_id_2: i64 = stmt
                                .query_row(params![], |row| row.get(0))
                                .expect("город организатора не найден");

                            let mut stmt = conn
                                .prepare(&format!("SELECT player_id_3 FROM schema_game_{} WHERE stage IS NULL ORDER BY id ASC
        LIMIT 1", game_id))
                                .expect("не удалось выбрать город организатора");

                            let player_id_3: i64 = stmt
                                .query_row(params![], |row| row.get(0))
                                .expect("город организатора не найден");

                            let mut stmt = conn
                                .prepare(&format!("SELECT player_id_4 FROM schema_game_{} WHERE stage IS NULL ORDER BY id ASC
        LIMIT 1", game_id))
                                .expect("не удалось выбрать город организатора");

                            let player_id_4: i64 = stmt
                                .query_row(params![], |row| row.get(0))
                                .expect("город организатора не найден");

                            // Создание вектора с идентификаторами игроков
                            let player_ids =
                                vec![player_id_1, player_id_2, player_id_3, player_id_4];

                            for &player_id in &player_ids {
                                let price_question = 0; //хардкодим поскольку не получаем из веб формы

                                //проверка уникальности ответа игрока (необходимо для исключения повторного начисления баллов при обновлении страницы браузера)
                                if let Err(_) = result_answer_unique(
                                    &conn,
                                    game_id,
                                    questions_pac_id,
                                    tour,
                                    round,
                                    player_id,
                                    question_id,
                                    price_question,
                                ) {
                                    //  println!("Ошибка при проверке уникальности ответа: {}", e);
                                    return changing_four_players(
                                        &conn,
                                        user_id,
                                        game_id,
                                        questions_pac_id,
                                    );
                                }

                                conn.execute(
                                    &format!("INSERT INTO game_{} (questions_pac_id, tour, round, player_id, question_id, score) VALUES (?, ?, ?, ?, ?, 0)", game_id),
                                    params![questions_pac_id, tour, round, player_id, question_id],
                                ).expect("Failed to insert player into the database");
                            }

                            update_stage_schema_game(game_id, question_id); //запускаем обновление stage в схеме игры

                            changing_four_players(&conn, user_id, game_id, questions_pac_id)
                            //запускает проверку смены раунда
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может проводить игры".to_string(),
                        };
                        Template::render("index", &context)
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваши права не определены".to_string(),
                    };
                    Template::render("index", &context)
                }
            }
        }
        Err(_) => {
            // Пользователь не аутентифицирован, перейдите на главную страницу
            let context = Context {
                header: "Стартовая страница".to_string(),
            };
            Template::render("index", &context)
        }
    }
}

// Функция для удаления дублирующихся записей (один игрок ответил не верно, остальные игроки не знают ответ)
fn delete_duplicate_records(game_id: i64) {
    let conn = establish_connection();

    //находим одинаковые player_id, у найденных player_id сравниваем question_id, если question_id одинаковые - удаляем ту строку у которой score=0
    let query = format!(
        "DELETE FROM game_{}
        WHERE (player_id, question_id) IN (
            SELECT player_id, question_id
            FROM game_{}
            GROUP BY player_id, question_id
            HAVING COUNT(*) > 1
        ) AND score = 0",
        game_id, game_id
    );

    conn.execute(&query, params![])
        .expect("Failed to execute delete query");
}

// Функция для определения смены четверки игроков (изменение раунда или тура)
fn changing_four_players(
    conn: &Connection,
    user_id: i64,
    game_id: i64,
    questions_pac_id: i64,
) -> Template {
    // Находим максимальный id среди строк, где stage=1
    let max_id: Option<i32> = conn
        .query_row(
            &format!(
                "SELECT MAX(id) FROM schema_game_{} WHERE stage = 1",
                game_id
            ),
            [],
            |row| row.get(0),
        )
        .expect("не удалось выбрать max_id в changing_round");

    // Проверяем, есть ли запись с stage=1 и её id равен 80, 160 или 240
    if let Some(max_id_value) = max_id {
        if vec![80, 160, 240, 270].contains(&max_id_value) {
            //удаляем все повторы ответов (если игрок ответил не правильно возникает повторение ответа на вопрос-отрицательный и нулевой)
            delete_duplicate_records(game_id);

            result_tour(&conn, game_id, user_id, questions_pac_id, max_id)
        } else {
            let query = format!(
                "SELECT t1.round, t2.round
        FROM schema_game_{game_id} t1
        JOIN schema_game_{game_id} t2
        ON t1.id != t2.id
        WHERE t1.stage = 1
            AND t2.stage IS NULL
        ORDER BY t1.id DESC, t2.id ASC
        LIMIT 1;",
                game_id = game_id
            );

            let mut stmt = conn.prepare(&query).expect("Failed to prepare query");

            let rounds_match: (Option<i64>, Option<i64>) = stmt
                .query_row(params![], |row| {
                    let round1: Option<i64> = row.get(0)?;
                    let round2: Option<i64> = row.get(1)?;
                    Ok((round1, round2))
                })
                .unwrap_or_else(|_err| {
                    // eprintln!("Failed to retrieve result: {}", err);
                    (Some(1), Some(1)) // Если t1.id None, приравниваем к 1
                });

            match rounds_match {
                (r1, r2) if r1 == r2 => game(user_id, game_id, questions_pac_id),
                _ => changing_round(&conn, game_id, user_id, questions_pac_id),
            }
        }
    } else {
        let context = Context {
            header: "max_id не определен. Сделайте скриншот и отправьте его администратору!"
                .to_string(),
        };
        Template::render("404", &context)
    }
}

//смена раунда игры
fn changing_round(
    conn: &Connection,
    game_id: i64,
    user_id: i64,
    questions_pac_id: i64,
) -> Template {
    //если это первый вопрос раунда и уже был отрицательный ответ игрока
    let mut stmt = conn
        .prepare(&format!(
            "SELECT score
                FROM game_{}
                ORDER BY id DESC
                LIMIT 1",
            game_id
        ))
        .expect("не удалось выбрать раунда");

    let last_score: i8 = stmt
        .query_row(params![], |row| row.get(0))
        .expect("номер раунда не найден ищи тут");

    if last_score == -10 {
        //Если последний ответ -10 баллов переходим на страницу game. Если не сделать эту проверку, то при случайном нажатии кнопки верно, после нажатия кнопки "не верно" загрузит страницу changing_round
        game(user_id, game_id, questions_pac_id)
    } else {
        let mut stmt = conn
            .prepare(&format!(
                "SELECT tour
                FROM schema_game_{}
                WHERE stage = 1
                ORDER BY id DESC
                LIMIT 1",
                game_id
            ))
            .expect("не удалось выбрать тур");

        let tour: i8 = stmt
            .query_row(params![], |row| row.get(0))
            .expect("номер тура не найден");

        let mut stmt = conn
            .prepare(&format!(
                "SELECT round
                FROM schema_game_{}
                WHERE stage = 1
                ORDER BY id DESC
                LIMIT 1",
                game_id
            ))
            .expect("не удалось выбрать раунда");

        let round: i8 = stmt
            .query_row(params![], |row| row.get(0))
            .expect("номер раунда не найден");

        let header_tour = "".to_string();

        let header_next_players = "".to_string();

        let city = get_organiser_city(user_id);

        let context = DataRound {
            header: city,
            tour,
            round,
            game_id,
            questions_pac_id,
            header_tour,
            header_next_players,
        };
        Template::render("changing_round", &context)
    }
}

//структура для данных финальной страницы игры
#[derive(Serialize)]
struct FinalPage {
    header: String,
    game_id: i64,
    questions_pac_id: i64,
    tour: i8,
    round: i8,
}

//смена тура
fn result_tour(
    conn: &Connection,
    game_id: i64,
    user_id: i64,
    questions_pac_id: i64,
    max_id: Option<i32>,
) -> Template {
    //если max_id=270 (последний вопрос игры сыгран), то
    if max_id == Some(270) {
        //загружаем финальную страницу с результатами игры
        let mut stmt = conn
            .prepare(&format!(
                "SELECT tour
                FROM schema_game_{}
                WHERE stage = 1
                ORDER BY id DESC
                LIMIT 1",
                game_id
            ))
            .expect("не удалось выбрать тур");

        let tour: i8 = stmt
            .query_row(params![], |row| row.get(0))
            .expect("номер тура не найден");

        let mut stmt = conn
            .prepare(&format!(
                "SELECT round
                FROM schema_game_{}
                WHERE stage = 1
                ORDER BY id DESC
                LIMIT 1",
                game_id
            ))
            .expect("не удалось выбрать раунда");

        let round: i8 = stmt
            .query_row(params![], |row| row.get(0))
            .expect("номер раунда не найден");

        let city = get_organiser_city(user_id);

        let context = FinalPage {
            header: city,
            game_id,
            questions_pac_id,
            tour,
            round,
        };
        Template::render("result_game", &context)
    } else {
        //если это первый вопрос раунда и уже был отрицательный ответ игрока
        let mut stmt = conn
            .prepare(&format!(
                "SELECT score
                FROM game_{}
                ORDER BY id DESC
                LIMIT 1",
                game_id
            ))
            .expect("не удалось выбрать раунда");

        let last_score: i8 = stmt
            .query_row(params![], |row| row.get(0))
            .expect("номер раунда не найден ищи тут");

        if last_score == -10 {
            //Если последний ответ -10 баллов переходим на страницу game. Если не сделать эту проверку, то при случайном нажатии кнопки верно, после нажатия кнопки "не верно" загрузит страницу changing_round
            game(user_id, game_id, questions_pac_id)
        } else {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT tour
                FROM schema_game_{}
                WHERE stage = 1
                ORDER BY id DESC
                LIMIT 1",
                    game_id
                ))
                .expect("не удалось выбрать тур");

            let tour: i8 = stmt
                .query_row(params![], |row| row.get(0))
                .expect("номер тура не найден");

            let mut stmt = conn
                .prepare(&format!(
                    "SELECT round
                FROM schema_game_{}
                WHERE stage = 1
                ORDER BY id DESC
                LIMIT 1",
                    game_id
                ))
                .expect("не удалось выбрать раунда");

            let round: i8 = stmt
                .query_row(params![], |row| row.get(0))
                .expect("номер раунда не найден");

            let city = get_organiser_city(user_id);

            if tour == 3 && round == 4 {
                let header_tour = "Отборочные туры окончены с результатами:".to_string();
                let header_next_players = "ФИНАЛИСТЫ:".to_string();

                let context = DataRound {
                    header: city,
                    tour,
                    round,
                    game_id,
                    questions_pac_id,
                    header_tour,
                    header_next_players,
                };
                Template::render("result_tour", &context)
            } else {
                let header_tour = format!("{} тур окончен с результатами:", tour).to_string();
                let header_next_players = "Приглашается следующая четвёрка игроков:".to_string();

                let context = DataRound {
                    header: city,
                    tour,
                    round,
                    game_id,
                    questions_pac_id,
                    header_tour,
                    header_next_players,
                };
                Template::render("result_tour", &context)
            }
        }
    }
}

//структура для передачи данных на страницу changing_round
#[derive(Serialize)]
struct DataRound {
    header: String,
    tour: i8,
    round: i8,
    game_id: i64,
    questions_pac_id: i64,
    header_tour: String,
    header_next_players: String,
}

//отправка четвёрки участников (ФИО и id и score) на веб страницу итогов раунда
#[get("/get_players_round_result/<game_id>/<tour>/<round>")]
pub async fn get_players_round_result(
    cookies: &CookieJar<'_>,
    game_id: i64,
    tour: i8,
    round: i8,
) -> Json<PlayersGameDataResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let connection = establish_connection();
            let all_players_game = query_players_round_result(&connection, game_id, tour, round);

            Json(all_players_game)
        }
        Err(_) => Json(PlayersGameDataResponse { players: vec![] }), // Вернуть объект QuestionsResponse с пустым массивом в случае ошибки
    }
}

// Функция для запроса четверки игроков закончивших раунд
fn query_players_round_result(
    conn: &Connection,
    game_id: i64,
    tour: i8,
    round: i8,
) -> PlayersGameDataResponse {
    // Запрос для player_id_1
    let sql_query_1 = format!(
        "SELECT
        g.player_id_1,
        p1.player_real_first_name as p1_first_name,
        p1.player_real_patronymic as p1_patronymic,
        p1.player_real_last_name as p1_last_name,
        (
            SELECT SUM(score)
            FROM game_{}
            WHERE player_id = g.player_id_1 AND tour = ? AND round = ?
        ) as total_score
    FROM schema_game_{} g
    LEFT JOIN players p1 ON g.player_id_1 = p1.player_id
    WHERE g.stage = 1
    ORDER BY g.id DESC
    LIMIT 1",
        game_id, game_id
    );

    let mut stmt_1 = conn
        .prepare(&sql_query_1)
        .expect("Failed to prepare query query_players_game");

    let players_data_iter_1 = stmt_1
        .query_map(params![tour, round], |row| {
            Ok(PlayersGameData {
                id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                game_id,
                total_score: row.get(4).unwrap_or(0), // Используйте unwrap_or, чтобы предоставить значение по умолчанию
            })
        })
        .expect("Failed to query query_players_game");

    // Запрос для player_id_2
    let sql_query_2 = format!(
        "SELECT
            g.player_id_2,
            p2.player_real_first_name as p2_first_name,
            p2.player_real_patronymic as p2_patronymic,
            p2.player_real_last_name as p2_last_name,
        (
            SELECT SUM(score)
            FROM game_{}
            WHERE player_id = g.player_id_2 AND tour = ? AND round = ?
        ) as total_score

        FROM schema_game_{} g
        LEFT JOIN players p2 ON g.player_id_2 = p2.player_id
        WHERE g.stage = 1
        ORDER BY g.id DESC
        LIMIT 1",
        game_id, game_id
    );

    let mut stmt_2 = conn
        .prepare(&sql_query_2)
        .expect("Failed to prepare query query_players_game");

    let players_data_iter_2 = stmt_2
        .query_map(params![tour, round], |row| {
            Ok(PlayersGameData {
                id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                game_id,
                total_score: row.get(4).unwrap_or(0), // Используйте unwrap_or, чтобы предоставить значение по умолчанию
            })
        })
        .expect("Failed to query query_players_game");

    // Запрос для player_id_3
    let sql_query_3 = format!(
        "SELECT
            g.player_id_3,
            p3.player_real_first_name as p3_first_name,
            p3.player_real_patronymic as p3_patronymic,
            p3.player_real_last_name as p3_last_name,
        (
            SELECT SUM(score)
            FROM game_{}
            WHERE player_id = g.player_id_3 AND tour = ? AND round = ?
        ) as total_score
        FROM schema_game_{} g
        LEFT JOIN players p3 ON g.player_id_3 = p3.player_id
        WHERE g.stage = 1
        ORDER BY g.id DESC
        LIMIT 1",
        game_id, game_id
    );

    let mut stmt_3 = conn
        .prepare(&sql_query_3)
        .expect("Failed to prepare query query_players_game");

    let players_data_iter_3 = stmt_3
        .query_map(params![tour, round], |row| {
            Ok(PlayersGameData {
                id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                game_id,
                total_score: row.get(4).unwrap_or(0), // Используйте unwrap_or, чтобы предоставить значение по умолчанию
            })
        })
        .expect("Failed to query query_players_game");

    // Запрос для player_id_4
    let sql_query_4 = format!(
        "SELECT
            g.player_id_4,
            p4.player_real_first_name as p4_first_name,
            p4.player_real_patronymic as p4_patronymic,
            p4.player_real_last_name as p4_last_name,
         (
            SELECT SUM(score)
            FROM game_{}
            WHERE player_id = g.player_id_4 AND tour = ? AND round = ?
        ) as total_score
        FROM schema_game_{} g
        LEFT JOIN players p4 ON g.player_id_4 = p4.player_id
        WHERE g.stage = 1
        ORDER BY g.id DESC
        LIMIT 1",
        game_id, game_id
    );

    let mut stmt_4 = conn
        .prepare(&sql_query_4)
        .expect("Failed to prepare query query_players_game");

    let players_data_iter_4 = stmt_4
        .query_map(params![tour, round], |row| {
            Ok(PlayersGameData {
                id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                game_id,
                total_score: row.get(4).unwrap_or(0), // Используйте unwrap_or, чтобы предоставить значение по умолчанию
            })
        })
        .expect("Failed to query query_players_game");

    // Собираем результаты
    let players_data_1: Vec<PlayersGameData> = players_data_iter_1
        .map(|players_data| players_data.unwrap())
        .collect();

    let players_data_2: Vec<PlayersGameData> = players_data_iter_2
        .map(|players_data| players_data.unwrap())
        .collect();

    let players_data_3: Vec<PlayersGameData> = players_data_iter_3
        .map(|players_data| players_data.unwrap())
        .collect();

    let players_data_4: Vec<PlayersGameData> = players_data_iter_4
        .map(|players_data| players_data.unwrap())
        .collect();

    // Объединяем результаты
    let mut players_data = players_data_1;
    players_data.extend(players_data_2);
    players_data.extend(players_data_3);
    players_data.extend(players_data_4);

    PlayersGameDataResponse {
        players: players_data,
    }
}

//определяем четырёх победителей в трёх раундах и вносим их в таблицу schema_game_{}
fn formation_finalists_players(game_id: i64) {
    let conn = establish_connection();

    // Находим максимальный id среди строк, где stage=1
    let max_id: i32 = conn
        .query_row(
            &format!(
                "SELECT MAX(id) FROM schema_game_{} WHERE stage = 1",
                game_id
            ),
            [],
            |row| row.get(0),
        )
        .expect("Не удалось определить max_id в formation_finalists_players");

    // Проверяем, есть ли запись с stage=1 и её id равен 240
    if max_id == 240 {
        //находим четверку игроков с максимальной суммой баллов
        let mut stmt = conn
            .prepare(&format!(
                "SELECT player_id,
                 SUM(CASE WHEN tour = 1 THEN score ELSE 0 END) as score_tour_1,
    SUM(CASE WHEN tour = 2 THEN score ELSE 0 END) as score_tour_2,
    SUM(CASE WHEN tour = 3 THEN score ELSE 0 END) as score_tour_3,
    SUM(score) as total_score
             FROM game_{}
             GROUP BY player_id
    ORDER BY total_score DESC, score_tour_3 DESC, score_tour_2 DESC, score_tour_1 DESC
             LIMIT 4;",
                game_id
            ))
            .expect(&format!(
                "Не удалось посчитать результат трёх раундов game_{}",
                game_id
            ));

        //собираем игроков в вектор
        let final_player_ids: Vec<i64> = stmt
            .query_map([], |row| {
                let player_id: i64 = row.get("player_id")?;
                Ok(player_id) //создаём кортеж всех player_id и их сумму score
            })
            .expect("Ошибка при выполнении запроса")
            .map(|result| result.unwrap())
            .collect();

        // Перемешиваем найденные player_id
        let mut rng = rand::thread_rng();
        let mut shuffled_player_ids = final_player_ids.clone();
        shuffled_player_ids.shuffle(&mut rng);

        //обновляем schema_game_{} добавляя финалистов игры
        let update_query = format!(
                "UPDATE schema_game_{} SET player_id_1 = ?, player_id_2 = ?, player_id_3 = ?, player_id_4 = ? WHERE id >= 241 AND id < 271",
                game_id
            );

        // Выполнение запроса на обновление
        conn.execute(
            &update_query,
            params![
                shuffled_player_ids.get(0).unwrap_or(&0),
                shuffled_player_ids.get(1).unwrap_or(&0),
                shuffled_player_ids.get(2).unwrap_or(&0),
                shuffled_player_ids.get(3).unwrap_or(&0),
            ],
        )
        .expect("Ошибка при выполнении запроса на обновление");

        query_players_qualifying_round_result(&conn, game_id);
    } else {
        //eprintln!("Запись с stage=1 не найдена или id меньше 240");
    }
}

//структура для передачи данных на страницу result_three_round
#[derive(Serialize, Debug, Clone)]
struct PlayersQualifyingGameData {
    id: i64,
    player_real_last_name: String,
    player_real_first_name: String,
    player_real_patronymic: String,
    score_tour_1: i64,
    score_tour_2: i64,
    score_tour_3: i64,
    final_tour: i64,
    total_score: i64,
}

#[derive(Serialize, Debug)]
pub struct PlayersQualifyingGameDataResponse {
    players: Vec<PlayersQualifyingGameData>,
}

//отправка результатов отборочных туров участников (ФИО и id и score) на веб страницу
#[get("/get_tour_result/<game_id>")]
pub async fn get_tour_result(
    cookies: &CookieJar<'_>,
    game_id: i64,
) -> Json<PlayersQualifyingGameDataResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let connection = establish_connection();
            let all_players_qualifying_rounds_game =
                query_players_qualifying_round_result(&connection, game_id);

            Json(all_players_qualifying_rounds_game)
        }
        Err(_) => Json(PlayersQualifyingGameDataResponse { players: vec![] }), // Вернуть объект QuestionsResponse с пустым массивом в случае ошибки
    }
}

//определяем результаты всех игроков по окончании тура
fn query_players_qualifying_round_result(
    conn: &Connection,
    game_id: i64,
) -> PlayersQualifyingGameDataResponse {
    // results_tour(&conn, game_id);
    // Запрос для сумм баллов по каждому player_id (подключаемся к game_{}, находим уникальные player_id, суммируем score
    //каждого игрока, подключаемся к таблице players, по player_id находим ФИО каждого игрока, сортируем в порядке возрастания score
    let sql_query = format!(
        "SELECT g.player_id, p.player_real_first_name, p.player_real_patronymic, p.player_real_last_name,
    SUM(CASE WHEN g.tour = 1 THEN g.score ELSE 0 END) as score_tour_1,
    SUM(CASE WHEN g.tour = 2 THEN g.score ELSE 0 END) as score_tour_2,
    SUM(CASE WHEN g.tour = 3 THEN g.score ELSE 0 END) as score_tour_3,
    SUM(CASE WHEN g.tour = 4 THEN g.score ELSE 0 END) as final_tour,
    SUM(g.score) as total_score
    FROM game_{} g
    JOIN players p ON g.player_id = p.player_id
    GROUP BY g.player_id
    ORDER BY total_score ASC, final_tour ASC, score_tour_3 ASC, score_tour_2 ASC, score_tour_1 ASC",
        game_id
    );

    let mut stmt = conn
        .prepare(&sql_query)
        .expect("Failed to prepare query query_players_game");

    // Получаем из базы данных player_id, player_real_first_name, player_real_patronymic, player_real_last_name и
    // total_score каждого игрока
    let players_data_iter = stmt
        .query_map(params![], |row| {
            Ok(PlayersQualifyingGameData {
                id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                score_tour_1: row.get(4).unwrap_or(0),
                score_tour_2: row.get(5).unwrap_or(0),
                score_tour_3: row.get(6).unwrap_or(0),
                final_tour: row.get(7).unwrap_or(0),
                total_score: row.get(8).unwrap_or(0),
            })
        })
        .expect("Failed to query query_players_game");

    // Собираем результаты в вектор кортежей
    let players_data: Vec<PlayersQualifyingGameData> = players_data_iter
        .map(|players_data| players_data.unwrap())
        .collect();

    PlayersQualifyingGameDataResponse {
        players: players_data,
    }
}

#[post("/end_game/<game_id>")]
pub async fn end_game(cookies: &CookieJar<'_>, game_id: i64) -> Result<Redirect, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => {
                    match role.as_str() {
                        "organiser" => {
                            //stage = 1 значит игра сыграна
                            let conn = establish_connection();
                            conn.execute(
                                "UPDATE register_games SET stage = 1 WHERE id = ?",
                                params![game_id],
                            )
                            .expect(
                                "Вставка questions_pac_id в таблицу questions_players не удалась",
                            );

                            update_statistic_players(game_id);

                            getting_game_results(game_id);

                            del_reg_and_schema_game(game_id).await;

                            Ok(Redirect::to("/log_in_page")) // перенаправляем организатора на страницу prepare_questions_pac
                        }
                        _ => {
                            // Пользователь не аутентифицирован, перейдите на главную страницу
                            let context = Context {
                                header: "Только организатор может проводить игры".to_string(),
                            };
                            Err(Template::render("index", &context))
                        }
                    }
                }
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваши права не определены".to_string(),
                    };
                    Err(Template::render("index", &context))
                }
            }
        }
        Err(_) => {
            // Пользователь не аутентифицирован, перейдите на главную страницу
            let context = Context {
                header: "Стартовая страница".to_string(),
            };
            Err(Template::render("index", &context))
        }
    }
}

//структура для players_list
#[derive(Serialize)]
struct PlayersListPage {
    header: String,
    game_id: i64,
}

//отправка списка игроков на страницу player_list
#[post("/player_list/<game_id>")]
pub fn player_list(cookies: &CookieJar<'_>, game_id: i64) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let city = get_organiser_city(user_id);

                        let context = PlayersListPage {
                            header: city,
                            game_id,
                        };

                        Template::render("players_list", &context)
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может просматривать игроков".to_string(),
                        };
                        Template::render("404", &context)
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваша роль не определена".to_string(),
                    };
                    Template::render("404", &context)
                }
            }
        }
        Err(_) => {
            // Обработка ошибок get_user_id_from_cookies
            let context = Context {
                header: "Ошибка при получении идентификатора пользователя".to_string(),
            };
            Template::render("404", &context)
        }
    }
}

//структура для передачи данных на страницу player_list
#[derive(Serialize, Debug, Clone)]
struct PlayersListData {
    player_id: i64,
    player_real_last_name: String,
    player_real_first_name: String,
    player_real_patronymic: String,
    player_real_phone_number: i64,
}

#[derive(Serialize, Debug)]
pub struct PlayersListDataResponse {
    players: Vec<PlayersListData>,
}

#[get("/get_player_list/<game_id>")]
pub fn get_player_list(cookies: &CookieJar<'_>, game_id: i64) -> Json<PlayersListDataResponse> {
    println!("запуск get_player_list {}", game_id);

    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let conn = establish_connection();

            let all_players_registration_game = query_players_registration_result(&conn, game_id);

            Json(all_players_registration_game)
        }
        Err(_) => Json(PlayersListDataResponse { players: vec![] }), // Вернуть объект QuestionsResponse с пустым массивом в случае ошибки
    }
}

//определяем результаты всех игроков по окончании тура
fn query_players_registration_result(conn: &Connection, game_id: i64) -> PlayersListDataResponse {
    // results_tour(&conn, game_id);
    // Запрос для сумм баллов по каждому player_id (подключаемся к game_{}, находим уникальные player_id, суммируем score
    //каждого игрока, подключаемся к таблице players, по player_id находим ФИО каждого игрока, сортируем в порядке возрастания score
    let sql_query = format!(
            "SELECT g.player_id, p.player_real_first_name, p.player_real_patronymic, p.player_real_last_name, p.player_real_phone_number
    FROM reg_game_{} g
    JOIN players p ON g.player_id = p.player_id
    WHERE g.player_id IS NOT NULL
    GROUP BY g.player_id",
            game_id
        );

    let mut stmt = conn
        .prepare(&sql_query)
        .expect("Failed to prepare query query_players_registration_result");

    // Получаем из базы данных player_id, player_real_first_name, player_real_patronymic, player_real_last_name и
    // total_score каждого игрока
    let players_data_iter = stmt
        .query_map(params![], |row| {
            Ok(PlayersListData {
                player_id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                player_real_phone_number: row.get(4)?,
            })
        })
        .expect("Failed to query query_players_registration_result");

    // Собираем результаты в вектор кортежей
    let players_data: Vec<PlayersListData> = players_data_iter
        .map(|players_data| players_data.unwrap())
        .collect();

    PlayersListDataResponse {
        players: players_data,
    }
}

//отправка списка зрителей на страницу player_list
#[get("/get_spectator_list/<game_id>")]
pub fn get_spectator_list(cookies: &CookieJar<'_>, game_id: i64) -> Json<PlayersListDataResponse> {
    println!("запуск get_spectator_list {}", game_id);

    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let conn = establish_connection();

            let all_players_registration_game = query_spectator_registration_result(&conn, game_id);

            Json(all_players_registration_game)
        }
        Err(_) => Json(PlayersListDataResponse { players: vec![] }), // Вернуть объект QuestionsResponse с пустым массивом в случае ошибки
    }
}

//определяем результаты всех игроков по окончании тура
fn query_spectator_registration_result(conn: &Connection, game_id: i64) -> PlayersListDataResponse {
    println!("запуск query_spectator_registration_result");
    // results_tour(&conn, game_id);
    // Запрос для сумм баллов по каждому player_id (подключаемся к game_{}, находим уникальные player_id, суммируем score
    //каждого игрока, подключаемся к таблице players, по player_id находим ФИО каждого игрока, сортируем в порядке возрастания score
    let sql_query = format!(
        "SELECT g.spectator_id, p.player_real_first_name, p.player_real_patronymic, p.player_real_last_name, p.player_real_phone_number
    FROM reg_game_{} g
    JOIN players p ON g.spectator_id = p.player_id
    WHERE g.spectator_id IS NOT NULL
    GROUP BY g.spectator_id",
        game_id
    );

    let mut stmt = conn
        .prepare(&sql_query)
        .expect("Failed to prepare query query_players_registration_result");

    // Получаем из базы данных player_id, player_real_first_name, player_real_patronymic, player_real_last_name и
    // total_score каждого игрока
    let players_data_iter = stmt
        .query_map(params![], |row| {
            Ok(PlayersListData {
                player_id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                player_real_phone_number: row.get(4)?,
            })
        })
        .expect("Failed to query query_players_registration_result");

    // Собираем результаты в вектор кортежей
    let players_data: Vec<PlayersListData> = players_data_iter
        .map(|players_data| players_data.unwrap())
        .collect();

    PlayersListDataResponse {
        players: players_data,
    }
}

//отправка списка резервных игроков на страницу player_list
#[get("/get_reserve_player_list/<game_id>")]
pub fn get_reserve_player_list(
    cookies: &CookieJar<'_>,
    game_id: i64,
) -> Json<PlayersListDataResponse> {
    println!("запуск get_reserve_player_list {}", game_id);

    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let conn = establish_connection();

            let all_players_registration_game =
                query_reserve_player_registration_result(&conn, game_id);

            Json(all_players_registration_game)
        }
        Err(_) => Json(PlayersListDataResponse { players: vec![] }), // Вернуть объект QuestionsResponse с пустым массивом в случае ошибки
    }
}

//определяем результаты всех игроков по окончании тура
fn query_reserve_player_registration_result(
    conn: &Connection,
    game_id: i64,
) -> PlayersListDataResponse {
    println!("запуск query_reserve_player_registration_result");
    // results_tour(&conn, game_id);
    // Запрос для сумм баллов по каждому player_id (подключаемся к game_{}, находим уникальные player_id, суммируем score
    //каждого игрока, подключаемся к таблице players, по player_id находим ФИО каждого игрока, сортируем в порядке возрастания score
    let sql_query = format!(
        "SELECT g.reserve_player_id, p.player_real_first_name, p.player_real_patronymic, p.player_real_last_name, p.player_real_phone_number
    FROM reg_game_{} g
    JOIN players p ON g.reserve_player_id = p.player_id
    WHERE g.reserve_player_id IS NOT NULL
    GROUP BY g.reserve_player_id",
        game_id
    );

    let mut stmt = conn
        .prepare(&sql_query)
        .expect("Failed to prepare query query_players_registration_result");

    // Получаем из базы данных player_id, player_real_first_name, player_real_patronymic, player_real_last_name и
    // total_score каждого игрока
    let players_data_iter = stmt
        .query_map(params![], |row| {
            Ok(PlayersListData {
                player_id: row.get(0)?,
                player_real_last_name: row.get(3)?,
                player_real_first_name: row.get(1)?,
                player_real_patronymic: row.get(2)?,
                player_real_phone_number: row.get(4)?,
            })
        })
        .expect("Failed to query query_players_registration_result");

    // Собираем результаты в вектор кортежей
    let players_data: Vec<PlayersListData> = players_data_iter
        .map(|players_data| players_data.unwrap())
        .collect();

    PlayersListDataResponse {
        players: players_data,
    }
}

#[post("/exclude_from_game/<game_id>/<player_id>")]
pub async fn exclude_from_game(cookies: &CookieJar<'_>, game_id: i64, player_id: i64) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            // Пользователь аутентифицирован, перейдите на страницу подготовки игры
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        delete_game_player(game_id, player_id).await;

                        let city = get_organiser_city(user_id);

                        let context = Context { header: city };

                        Template::render("prepare_game", &context)
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может проводить игры".to_string(),
                        };
                        Template::render("index", &context)
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваши права не определены".to_string(),
                    };
                    Template::render("index", &context)
                }
            }
        }
        Err(_) => {
            // Пользователь не аутентифицирован, перейдите на главную страницу
            let context = Context {
                header: "Стартовая страница".to_string(),
            };
            Template::render("index", &context)
        }
    }
}

//удаление игры организатором
#[post("/del_game/<game_id>")]
pub async fn del_game(cookies: &CookieJar<'_>, game_id: i64) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            // Пользователь аутентифицирован, перейдите на страницу подготовки игры
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        // Выполняем del_game_bot синхронно
                        if let Err(err) = spawn_blocking(move || {
                            tokio::runtime::Runtime::new()
                                .unwrap()
                                .block_on(del_game_bot(game_id))
                        })
                        .await
                        {
                            eprintln!("Failed to execute del_game_bot: {}", err);
                            // Обработка ошибки, если необходимо
                        }

                        let conn = establish_connection();

                        let mut stmt = conn
                            .prepare("UPDATE register_games SET stage = 2, package_id = NULL WHERE id = ?")
                            .expect("не удалось обновить статус игры в таблице register_games");

                        stmt.execute(params![game_id])
                            .expect("не удалось обновить статус игры в del_game");

                        let mut stmt = conn
                            .prepare(&format!("DROP TABLE IF EXISTS reg_game_{}", game_id))
                            .expect(&format!("не удалось удалить таблицу reg_game_{}", game_id));

                        stmt.execute(params![]).expect(&format!(
                            "Не удалось удалить таблицу reg_game_{} в del_game",
                            game_id
                        ));

                        let city = get_organiser_city(user_id);

                        let context = Context { header: city };

                        Template::render("prepare_game", &context)
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может проводить игры".to_string(),
                        };
                        Template::render("index", &context)
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваши права не определены".to_string(),
                    };
                    Template::render("index", &context)
                }
            }
        }
        Err(_) => {
            // Пользователь не аутентифицирован, перейдите на главную страницу
            let context = Context {
                header: "Стартовая страница".to_string(),
            };
            Template::render("index", &context)
        }
    }
}

#[post("/cancellation_last_accrual_points/<game_id>/<questions_pac_id>")]
pub async fn cancellation_last_accrual_points(
    cookies: &CookieJar<'_>,
    game_id: i64,
    questions_pac_id: i64,
) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            // Пользователь аутентифицирован, перейдите на страницу подготовки игры
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let conn = establish_connection();

                        // Подготовка SQL-запроса для выбора последних четырех записей
                        let sql_query = format!(
                            "SELECT score FROM game_{} ORDER BY id DESC LIMIT 4",
                            game_id
                        );

                        // Выполнение SQL-запроса и получение результатов
                        let scores: Result<Vec<i8>, _> = conn
                            .prepare(&sql_query)
                            .and_then(|mut stmt| {
                                stmt.query_map(params![], |row| row.get(0))
                                    .map_err(|err| {
                                        eprintln!("Ошибка при выполнении SQL-запроса: {:?} в cancellation_last_accrual_points", err);
                                        err
                                    })
                                    .map(|mapped_rows| mapped_rows.map(|row| row.unwrap()).collect())
                            });

                        // Проверка, что все значения в последних четырех записях отрицательные
                        match scores {
                            Ok(scores) => {
                                let all_negative = scores.iter().all(|&score| score < 0);

                                if all_negative {
                                    del_last_rec_schema_game(game_id);

                                    game(user_id, game_id, questions_pac_id)
                                } else {
                                    // Ваш код для обработки случая, когда последняя запись отрицательная
                                    let mut stmt = conn
                                        .prepare(&format!("SELECT id, score FROM game_{} ORDER BY id DESC LIMIT 1", game_id))
                                        .expect(&format!("не удалось определить id и score в game_{} в cancellation_last_accrual_points", game_id));

                                    let (last_id, last_score): (i32, i8) = stmt
                                        .query_row(params![], |row| Ok((
                                            row.get(0)?,
                                            row.get(1)?))
                                        )
                                        .expect(&format!("не удалось определить score в game_{} в cancellation_last_accrual_points", game_id));

                                    if last_score < 0 {
                                        conn
                                            .execute(&format!("DELETE FROM game_{} WHERE id = ?", game_id),
                                                     params![last_id])
                                            .expect(&format!("не удалось удалить отрицательное начисление баллов в таблице game_{}", game_id));

                                        game(user_id, game_id, questions_pac_id)
                                    } else {
                                        del_last_rec_schema_game(game_id);

                                        game(user_id, game_id, questions_pac_id)
                                    }
                                }
                            }
                            Err(err) => {
                                eprintln!("Ошибка при выполнении SQL-запроса: {:?}", err);

                                let context = Context {
                                    header: format!("Сделайте скриншот страницы и отправьте администратору {:?}", err).to_string(), };

                                Template::render("index", &context)
                            }
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может проводить игры".to_string(),
                        };
                        Template::render("index", &context)
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваши права не определены".to_string(),
                    };
                    Template::render("index", &context)
                }
            }
        }
        Err(_) => {
            // Пользователь не аутентифицирован, перейдите на главную страницу
            let context = Context {
                header: "Стартовая страница".to_string(),
            };
            Template::render("index", &context)
        }
    }
}

//удаляем последнюю запись из schema_game и ответы всех игроков на последний вопрос
fn del_last_rec_schema_game(game_id: i64) {
    let conn = establish_connection();

    let mut stmt = conn
        .prepare(&format!(
            "SELECT question_id FROM schema_game_{} WHERE stage = 1 ORDER BY id DESC LIMIT 1",
            game_id
        ))
        .expect("не удалось выбрать question_id в cancellation_last_accrual_points");

    let question_id: i32 = stmt
        .query_row(params![], |row| row.get(0))
        .expect("номер вопроса в cancellation_last_accrual_points не найден");

    conn.execute(
        &format!("DELETE FROM game_{} WHERE question_id = ?", game_id),
        params![question_id],
    )
    .expect(&format!(
        "не удалось найти question_id в таблице game_{}",
        game_id
    ));

    conn.execute(
        &format!(
            "UPDATE schema_game_{} SET stage = NULL WHERE question_id = ?",
            game_id
        ),
        params![question_id],
    )
    .expect("не удалось обновить статус игры в таблице register_games");
}
