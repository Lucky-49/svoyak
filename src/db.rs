/* В этом файле содержаться функции, связанные со взаимодействием приложения с базой данных.
Это может включать в себя функции для установления соединения с базой данных, выполнения
запросов, обработки результатов запросов и другие функции, связанные с базой данных. */

use crate::tg::bot;
use crate::tg::bot::{free_space_game_bot, sending_game_results, PlayerProfile, RealPlayerData};
use crate::web::users::get_user_id_from_cookies;
use rocket::http::CookieJar;
use rocket::serde::json::Json;
use rusqlite::config::DbConfig;
use rusqlite::{params, OptionalExtension, Result};
use rusqlite::{Connection, Error};
use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use teloxide::prelude::ChatId;
use tokio::task::spawn_blocking;
use crate::tg::token::{SUPERADMIN_CITY, SUPERADMIN_PASSWORD, SUPERADMIN_ROLE, SUPERADMIN_USERNAME};

pub async fn run_db() -> Result<(), Box<dyn std::error::Error>> {
    //создание базы данных в директории проекта
    let current_dir = env::current_dir()?;
    let database_path = current_dir.join("Users.db");
    let schema_file_path = current_dir.join("src").join("schema.sql");
    let sql_file_contents = read_sql_from_file(schema_file_path.to_str().expect("Invalid path"));

    if !Path::new(&database_path).exists() {
        // Открыть или создать базу данных с флагами чтения-записи и создания
        let conn = Connection::open_with_flags(
            &database_path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE | rusqlite::OpenFlags::SQLITE_OPEN_CREATE,
        )
        .expect("Failed to open or create database");

        // Устанавливаем поддержку внешнего ключа
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY, true)?;

        // Выполняем пакет SQL
        conn.execute_batch(&sql_file_contents)?;

        conn.execute(
            "INSERT INTO users (role, username, password, city) VALUES (?1, ?2, ?3, ?4)",
            params![SUPERADMIN_ROLE, SUPERADMIN_USERNAME, SUPERADMIN_PASSWORD, SUPERADMIN_CITY]
        ).expect("Не удалось вставить данные superadmin");

        // Соединение будет закрыто автоматически, когда оно выйдет за пределы области видимости
    }

    Ok(())
}

// функция осуществляет чтение содержимого файла с SQL-запросами из указанного пути (path)
pub fn read_sql_from_file(path: &str) -> String {
    let mut file = File::open(path).expect("Файл базы данных не найден");
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .expect("Не удалось прочитать файл базы данных");

    contents
}

/// Функция, возвращающая подключение к базе данных SQLite
pub fn establish_connection() -> Connection {
    let current_dir = env::current_dir().expect("Не удалось получить текущий каталог");
    let database_path = current_dir.join("Users.db");

    Connection::open(&database_path).expect("подключение к базе данных не удалось")
}

//запись данных об игроке при первом подключении к боту
pub async fn write_player_data_to_db(player_data: &bot::PlayerData) -> Result<(), Error> {
    let conn = establish_connection();

    let player_id = player_data.player_id.unwrap_or(0);

    let existing_player = conn.query_row(
        "SELECT player_id FROM players WHERE player_id = ?1",
        params![player_id],
        |row| row.get::<usize, i64>(0),
    );

    match existing_player {
        Ok(_) => {
            // Если запись существует, обновляем данные
            conn.execute(
                "UPDATE players SET player_name = ?1, player_first_name = ?2,
                player_last_name = ?3, chat_id = ?4 WHERE player_id = ?5",
                params![
                    player_data.player_name.as_ref().unwrap_or(&"".to_string()),
                    player_data
                        .player_first_name
                        .as_ref()
                        .unwrap_or(&"".to_string()),
                    player_data
                        .player_last_name
                        .as_ref()
                        .unwrap_or(&"".to_string()),
                    player_data.chat_id.expect("REASON").to_string(),
                    player_id,
                ],
            )?;
        }
        Err(_) => {
            conn.execute(
                "INSERT INTO players (player_id, player_name, player_first_name, player_last_name, chat_id) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
            player_data.player_id.unwrap_or_default(),
            player_data.player_name.as_ref().unwrap_or(&"".to_string()),
            player_data.player_first_name.as_ref().unwrap_or(&"".to_string()),
            player_data.player_last_name.as_ref().unwrap_or(&"".to_string()),
            player_data.chat_id.expect("REASON").to_string(),
        ],
            )?;
        }
    }
    Ok(())
}

//запись данных об игроке при регистрации в сервисе
pub async fn write_real_player_data_to_db(
    player_id: i64,
    player_real_first_name: Option<String>,
    player_real_patronymic: Option<String>,
    player_real_last_name: Option<String>,
    player_real_location: Option<String>,
    player_real_phone_number: Option<i64>,
) -> Result<(), Error> {
    let conn = establish_connection();

    conn.execute(
        "UPDATE players SET
            player_real_first_name = ?1,
            player_real_patronymic = ?2,
            player_real_last_name = ?3,
            player_real_location = ?4,
            player_real_phone_number = ?5
        WHERE player_id = ?6",
        params![
            player_real_first_name,
            player_real_patronymic,
            player_real_last_name,
            player_real_location,
            player_real_phone_number,
            player_id,
        ],
    )?;

    Ok(())
}

#[derive(Debug)]
pub struct PlayerStatistic {
    pub player_play_games: i64,
    pub player_win_games: i64,
    pub player_correct_answer: i64,
    pub player_incorrect_answer: i64,
    pub player_total_score: i64,
}

//получение статистики игрока
pub async fn get_player_statistic(player_id: i64) -> Result<PlayerStatistic> {
    let conn = establish_connection();

    let mut stmt = conn
        .prepare("SELECT player_play_games, player_win_games, player_correct_answer, player_incorrect_answer, player_total_score FROM players WHERE player_id = ?")
        .expect("ошибка получения данных в gey_player_statistic");

    let statistic_data: PlayerStatistic = stmt
        .query_row(params![player_id], |row| {
            Ok(PlayerStatistic {
                player_play_games: row.get(0).unwrap_or(0),
                player_win_games: row.get(1).unwrap_or(0),
                player_correct_answer: row.get(2).unwrap_or(0),
                player_incorrect_answer: row.get(3).unwrap_or(0),
                player_total_score: row.get(4).unwrap_or(0),
            })
        })
        .unwrap_or_else(|e| {
            panic!(
                "ошибка при выполнении запроса в get_player_statistic: {}",
                e
            )
        });

    Ok(statistic_data)
}

//получение данных об игроке
pub async fn get_player_profile() -> Result<Vec<PlayerProfile>> {
    let conn = establish_connection();

    let mut stmt = conn.prepare(
        "SELECT player_id, chat_id, player_real_first_name, player_real_patronymic,\
        player_real_last_name, player_real_location, player_real_phone_number FROM players",
    )?;

    let player_iter = stmt.query_map(params![], |row| {
        let player_id: i64 = row.get(0)?; //Получаем значение из базы данных

        let chat_id_i64: i64 = row.get(1)?; // Получаем значение chat_id из базы данных как i64
        let chat_id = ChatId(i64::from(chat_id_i64));

        Ok(PlayerProfile {
            player_id: Some(player_id),
            chat_id: Some(chat_id),
            player_real_first_name: row.get(2)?,
            player_real_patronymic: row.get(3)?,
            player_real_last_name: row.get(4)?,
            player_real_location: row.get(5)?,
            player_real_phone_number: row.get(6)?,
        })
    })?;

    let mut users = Vec::new();
    for user in player_iter {
        users.push(user?);
    }
    Ok(users)
}

pub async fn add_to_single_question_db(
    //запись одиночного вопроса
    player_id: Option<i64>,
    player_single_question: String,
    player_single_answer_question: &str,
) {
    let mut conn = establish_connection();

    //начинаем транзакцию
    let transaction = conn.transaction().expect("Failed to start transaction");

    //выполняем запрос внутри транзакции
    let result = transaction
    .execute(
        "INSERT INTO questions_players (player_id, player_question, player_answer_question) VALUES (?1, ?2, ?3)",
        params![player_id, player_single_question, player_single_answer_question],
    ).expect("ошибка записи одиночного вопроса игрока в add_to_single_question_db");

    //проверяем результат выполнения запроса
    if result > 0 {
        //если все успешно - фиксируем транзакцию
        transaction
            .commit()
            .expect("Failed to commit transaction add_to_single_question_db");
    } else {
        //в случае ошибка отменяем транзакцию
        transaction
            .rollback()
            .expect("Failed to rollback transaction add_to_single_question_db");
        eprintln!(
            "Error inserting data for player_id {:?}: {} in rec_pre_reg_player",
            player_id, result
        );
    }
}

pub async fn add_to_multi_question_db(
    //запись мультивопроса
    player_id: Option<i64>,
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
    player_fifth_answer_question: String,
) {
    let mut conn = establish_connection();

    let transaction_first = conn
        .transaction()
        .expect("Failed to start transaction_first");

    //выполняем запрос внутри транзакции
    let result_first = transaction_first
        .execute(
        "INSERT INTO questions_players (player_id, player_topic_five_questions, player_question, player_answer_question) VALUES (?1, ?2, ?3, ?4)",
        params![player_id, player_topic_multi_question, player_first_question, player_first_answer_question],
    ).expect("не удалось записать первый вопрос в add_to_multi_question_db");

    //проверяем результат выполнения запроса
    if result_first > 0 {
        //если все успешно - фиксируем транзакцию
        transaction_first
            .commit()
            .expect("Failed to commit transaction_first");
    } else {
        //в случае ошибка отменяем транзакцию
        transaction_first
            .rollback()
            .expect("Failed to rollback transaction_first");
        eprintln!(
            "Error inserting data for player_id {:?}: {} in add_to_multi_question_db",
            player_id, result_first
        );
    }

    let transaction_second = conn
        .transaction()
        .expect("Failed to start transaction_second");

    //выполняем запрос внутри транзакции
    let result_second = transaction_second
    .execute(
        "INSERT INTO questions_players (player_id, player_topic_five_questions, player_question, player_answer_question) VALUES (?1, ?2, ?3, ?4)",
        params![player_id, player_topic_multi_question, player_second_question, player_second_answer_question],
    ).expect("не удалось записать второй вопрос в add_to_multi_question_db");

    //проверяем результат выполнения запроса
    if result_second > 0 {
        //если все успешно - фиксируем транзакцию
        transaction_second
            .commit()
            .expect("Failed to commit transaction_second");
    } else {
        //в случае ошибка отменяем транзакцию
        transaction_second
            .rollback()
            .expect("Failed to rollback transaction_second");
        eprintln!(
            "Error inserting data for player_id {:?}: {} in add_to_multi_question_db",
            player_id, result_second
        );
    }

    let transaction_third = conn
        .transaction()
        .expect("Failed to start transaction_third");

    //выполняем запрос внутри транзакции
    let result_third = transaction_third
    .execute(
        "INSERT INTO questions_players (player_id, player_topic_five_questions, player_question, player_answer_question) VALUES (?1, ?2, ?3, ?4)",
        params![player_id, player_topic_multi_question, player_third_question, player_third_answer_question],
    ).expect("не удалось записать третий вопрос в add_to_multi_question_db");

    //проверяем результат выполнения запроса
    if result_third > 0 {
        //если все успешно - фиксируем транзакцию
        transaction_third
            .commit()
            .expect("Failed to commit transaction_third");
    } else {
        //в случае ошибка отменяем транзакцию
        transaction_third
            .rollback()
            .expect("Failed to rollback transaction_third");
        eprintln!(
            "Error inserting data for player_id {:?}: {} in add_to_multi_question_db",
            player_id, result_third
        );
    }

    let transaction_fourth = conn
        .transaction()
        .expect("Failed to start transaction_fourth");

    //выполняем запрос внутри транзакции
    let result_fourth = transaction_fourth
    .execute(
        "INSERT INTO questions_players (player_id, player_topic_five_questions, player_question, player_answer_question) VALUES (?1, ?2, ?3, ?4)",
        params![player_id, player_topic_multi_question, player_fourth_question, player_fourth_answer_question],
    ).expect("не удалось записать четвертый вопрос в add_to_multi_question_db");

    //проверяем результат выполнения запроса
    if result_fourth > 0 {
        //если все успешно - фиксируем транзакцию
        transaction_fourth
            .commit()
            .expect("Failed to commit transaction_fourth");
    } else {
        //в случае ошибка отменяем транзакцию
        transaction_fourth
            .rollback()
            .expect("Failed to rollback transaction_fourth");
        eprintln!(
            "Error inserting data for player_id {:?}: {} in add_to_multi_question_db",
            player_id, result_fourth
        );
    }

    let transaction_fifth = conn
        .transaction()
        .expect("Failed to start transaction_fifth");

    //выполняем запрос внутри транзакции
    let result_fifth = transaction_fifth
    .execute(
        "INSERT INTO questions_players (player_id, player_topic_five_questions, player_question, player_answer_question) VALUES (?1, ?2, ?3, ?4)",
        params![player_id,player_topic_multi_question, player_fifth_question, player_fifth_answer_question],
    ).expect("не удалось записать пятый вопрос в add_to_multi_question_db");

    //проверяем результат выполнения запроса
    if result_fifth > 0 {
        //если все успешно - фиксируем транзакцию
        transaction_fifth
            .commit()
            .expect("Failed to commit transaction_fifth");
    } else {
        //в случае ошибка отменяем транзакцию
        transaction_fifth
            .rollback()
            .expect("Failed to rollback transaction_fifth");
        eprintln!(
            "Error inserting data for player_id {:?}: {} in add_to_multi_question_db",
            player_id, result_fifth
        );
    }
}

struct NewGameTable {
    user_id: i64,
    game_day: String,
    game_time: String,
    game_location: String,
    price_player: i32,
    price_spectator: i32,
}

//создание таблицы объявленной игры
pub fn create_game_table(
    //создаем запись в таблице register_questions_pac и таблицу с id этой записи
    connection: &Connection,
    user_id: i64,
    game_day: String,
    game_time: String,
    game_location: String,
    price_player: i32,
    price_spectator: i32,
    seats_spectator: i8,
) -> std::result::Result<i64, Error> {
    let new_game_table = NewGameTable {
        user_id,
        game_day: game_day.clone(),
        game_time: game_time.clone(),
        game_location: game_location.clone(),
        price_player,
        price_spectator,
    };

    // Проверка уникальности названия пакета вопросов
    let is_name_game_table_unique: bool = connection
        .query_row(
            "SELECT COUNT(*) FROM register_games WHERE user_id = ? AND game_day = ? AND game_time = ? AND game_location = ?",
            params![&new_game_table.user_id,
            &new_game_table.game_day,
            &new_game_table.game_time,
            &new_game_table.game_location
            ],
            |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
        )
        .expect("не удалось выполнить запрос проверки уникальности create_game_table");

    // Игра не уникальна, возвращаем ошибку
    if !is_name_game_table_unique {
        println!("В одной локации не может быть назначена игра на то же время, прекращаем выполнение create_game_table");
        return Err(rusqlite::Error::QueryReturnedNoRows.into());
    }

    connection
        .execute(
            "INSERT INTO register_games
            (user_id, game_day, game_time, game_location,  price_player,
            price_spectator, stage, seats_spectator) VALUES (?, ?, ?, ?, ?, ?, 0, ?)",
            params![
                user_id,
                game_day,
                game_time,
                game_location,
                price_player,
                price_spectator,
                seats_spectator
            ],
        )
        .expect("не удалось вставить данные в таблицу register_games");

    // Получаем id последней записи с конкретным user_id
    let last_inserted_id: i64 = connection
        .query_row(
            "SELECT id FROM register_games WHERE user_id = ? ORDER BY id DESC LIMIT 1",
            params![&user_id],
            |row| row.get(0),
        )
        .unwrap_or_default();

    // Создаем таблицу с использованием id из register_games
    let table_name = format!("reg_game_{}", last_inserted_id);

    connection.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id INTEGER PRIMARY KEY,
                player_id INTEGER,
                reserve_player_id INTEGER,
                spectator_id INTEGER
            )",
            table_name
        ),
        [],
    )?;

    Ok(last_inserted_id)
}

//запись игрока в базу данных (регистрация на игру)
pub fn reg_game_player(player_id: i64, game_id: i64) -> Result<(), ()> {
    let connection = establish_connection();

    // Подсчет общего количества записей в столбце player_id
    let count: i8 = connection
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM reg_game_{} WHERE player_id IS NOT NULL",
                game_id
            ),
            [],
            |row| row.get(0),
        )
        .expect("не удалось выполнить запрос"); // Изменил эту строку

    // Вывод общего количества записей в терминал
    println!("Total number of records for player_id: {}", count);

    if count < 16 {
        //проверяем не зарегистрирован ли player_id зрителем
        let spectator_id: bool = connection
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM reg_game_{} WHERE spectator_id = ?",
                    game_id
                ),
                params![player_id],
                |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
            )
            .expect("не удалось выполнить запрос проверки уникальности reg_game_spectator");

        //если player_id уже зарегистрирован зрителем
        if !spectator_id {
            println!("player_id уже зарегистрирован зрителем");

            // Подключение к таблице game_{}
            let delete_result = connection.execute(
                &format!("DELETE FROM reg_game_{} WHERE spectator_id = ?", game_id),
                params![player_id],
            );

            match delete_result {
                Ok(rows_affected) => {
                    if rows_affected > 0 {
                        println!("Запись успешно удалена");
                    } else {
                        println!("Запись не найдена");
                    }
                }
                Err(err) => {
                    eprintln!("Ошибка при удалении записи: {:?}", err);
                    // Обработка ошибки, если необходимо
                }
            }
        }

        //проверяем не зарегистрирован ли player_id в резерв
        let reserve_player_id: bool = connection
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM reg_game_{} WHERE reserve_player_id = ?",
                    game_id
                ),
                params![player_id],
                |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
            )
            .expect("не удалось выполнить запрос проверки уникальности reg_game_reserve_player_id");

        //если player_id уже зарегистрирован в резерв
        if !reserve_player_id {
            println!("player_id уже зарегистрирован в резерв");

            // Подключение к таблице game_{}
            let delete_result = connection.execute(
                &format!(
                    "DELETE FROM reg_game_{} WHERE reserve_player_id = ?",
                    game_id
                ),
                params![player_id],
            );

            match delete_result {
                Ok(rows_affected) => {
                    if rows_affected > 0 {
                        println!("Запись успешно удалена");
                    } else {
                        println!("Запись не найдена");
                    }
                }
                Err(err) => {
                    eprintln!("Ошибка при удалении записи: {:?}", err);
                    // Обработка ошибки, если необходимо
                }
            }
        }

        // Вставляем данные в таблицу game_{}
        connection
            .execute(
                &format!("INSERT INTO reg_game_{} (player_id) VALUES (?)", game_id),
                params![player_id],
            )
            .expect(&format!(
                "не удалось вставить данные в таблицу reg_game_{}",
                game_id
            ));
        Ok(())
    } else {
        //проверяем, записан ли игрок в резерв
        let reserve_player_id: bool = connection
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM reg_game_{} WHERE reserve_player_id = ?",
                    game_id
                ),
                params![player_id],
                |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
            )
            .expect("не удалось выполнить запрос проверки уникальности reg_game_spectator");

        if !reserve_player_id {
            Err(())
        } else {
            // Вставляем данные в таблицу game_{}
            connection
                .execute(
                    &format!(
                        "INSERT INTO reg_game_{} (reserve_player_id) VALUES (?)",
                        game_id
                    ),
                    params![player_id],
                )
                .expect(&format!(
                    "не удалось вставить данные в таблицу reg_game_{}",
                    game_id
                ));
            Err(())
        }
    }
}

//запись зрителя в базу данных
pub fn reg_game_spectator(player_id: i64, game_id: i64) -> Result<(), &'static str> {
    let connection = establish_connection();

    let seats_spectator: i8 = connection
        .query_row(
            "SELECT seats_spectator FROM register_games WHERE id = ?",
            params![game_id],
            |row| row.get(0),
        )
        .expect("не удалось выбрать количество зрительных мест reg_game_spectator");

    // Подсчет общего количества записей в столбце spectator_id
    let count_all: i8 = connection
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM reg_game_{} WHERE spectator_id IS NOT NULL",
                game_id
            ),
            [],
            |row| row.get(0),
        )
        .expect("не удалось выполнить запрос"); // Изменил эту строку

    // Вывод общего количества записей в терминал
    println!("Total number of records for spectator_id: {}", count_all);

    if count_all < seats_spectator {
        //проверка наличия регистрации player_id зрителем
        let player_id_unique: bool = connection
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM reg_game_{} WHERE spectator_id = ?",
                    game_id
                ),
                params![player_id],
                |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
            )
            .expect("не удалось выполнить запрос проверки уникальности reg_game_spectator");

        //если player_id уже зарегистрирован зрителем
        if !player_id_unique {
            println!("player_id уже зарегистрирован зрителем");
            return Err("Зритель уже зарегистрирован");
        }

        connection
            .execute(
                &format!("INSERT INTO reg_game_{} (spectator_id) VALUES (?)", game_id),
                params![player_id],
            )
            .expect(&format!(
                "не удалось вставить данные в таблицу reg_game_{}",
                game_id
            ));
        Ok(())
    } else {
        Err("Мест нет")
    }
}

//удаляем игрока из таблицы reg_game_{}
pub async fn delete_game_player(game_id: i64, player_id: i64) {
    println!("запускаем delete_game_player");
    let connection = establish_connection();
    connection
        .execute(
            &format!("DELETE FROM reg_game_{} WHERE player_id = ?", game_id),
            params![player_id],
        )
        .expect("удалить регистрацию на игру player_id не удалось");

    // отправить сообщение всем reserve_player_id - освободилось место в игре
   let _ = spawn_blocking(move || {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(free_space_game_bot(game_id))
    })
    .await
    .expect("Failed to spawn blocking task");
}

#[derive(Debug)]
struct UpdatePlayerStatistic {
    player_id: i64,
    player_play_games: i64,
    player_correct_answer: i64,
    player_incorrect_answer: i64,
    player_total_score: i64,
}

//обновляем статистику игроков в таблице players
pub fn update_statistic_players(game_id: i64) {
    println!("запуск update_statistic_players");

    let conn = establish_connection();

    let sql_query = format!(
        "SELECT
        player_id,
        SUM(score) as total_score,
        COUNT(CASE WHEN score > 0 THEN 1 END) as positive_scores,
        COUNT(CASE WHEN score < 0 THEN 1 END) as negative_scores
    FROM game_{}
    WHERE score <> 0
    GROUP BY player_id",
        game_id
    );

    let mut stmt = conn
        .prepare(&sql_query)
        .expect("Failed to prepare query rec_result_table_players");

    // Получаем из базы данных player_id, score_final_tour и total_score каждого игрока
    let players_data_iter = stmt
        .query_map(params![], |row| {
            Ok((
                row.get(0).unwrap_or(0), //player_id
                row.get(1).unwrap_or(0), //total_score
                row.get(2).unwrap_or(0), //count_correct_answer
                row.get(3).unwrap_or(0), //count_incorrect_answer
            ))
        })
        .expect("Failed to query query_players_game");

    // Далее вы можете обрабатывать результаты запроса, например, выводить их в терминал
    for player_data in players_data_iter {
        match player_data {
            Ok((player_id, total_score, count_correct_answer, count_incorrect_answer)) => {
                // Запрос к таблице players для каждого player_id
                let mut stmt = conn
                    .prepare("SELECT player_play_games, player_correct_answer, player_incorrect_answer, player_total_score FROM players WHERE player_id = ?")
                    .expect("не удалось выбрать player_id в rec_result_table_players");

                let all_game_result_players = stmt
                    .query_map(params![player_id], |row| {
                        Ok(UpdatePlayerStatistic {
                            player_id,
                            player_play_games: row.get(0).unwrap_or(0) + 1,
                            player_correct_answer: row.get(1).unwrap_or(0) + count_correct_answer,
                            player_incorrect_answer: row.get(2).unwrap_or(0)
                                + count_incorrect_answer,
                            player_total_score: row.get(3).unwrap_or(0) + total_score,
                        })
                    })
                    .expect(
                        "не удалось создать структуру PlayerStatistic в rec_result_table_players",
                    );

                for result in all_game_result_players {
                    if let Ok(player_stat) = result {
                        conn.execute(
                            "UPDATE players SET
                                player_play_games = ?,
                                player_correct_answer = ?,
                                player_incorrect_answer = ?,
                                player_total_score = ?
                                WHERE player_id = ?",
                            params![
                                player_stat.player_play_games,
                                player_stat.player_correct_answer,
                                player_stat.player_incorrect_answer,
                                player_stat.player_total_score,
                                player_id,
                            ],
                        )
                        .expect("не удалось обновить данные после игры rec_result_table_players");
                    } else {
                        println!("что-то пошло не так в rec_result_table_players");
                    }
                }
            }
            Err(err) => {
                eprintln!("Error processing player data: {}", err);
            }
        }
    }

    //определяем player_id по сумме значений score из четвертого тура (финала)
    let sql_query = format!(
        "SELECT
    player_id,
    SUM(CASE WHEN tour = 4 THEN score ELSE 0 END) as final_tour
    FROM game_{}
    GROUP BY player_id
    ORDER BY final_tour DESC
    LIMIT 1",
        game_id
    );

    let mut stmt = conn
        .prepare(&sql_query)
        .expect("Failed to prepare query rec_result_table_players");

    // Получаем из базы данных player_id победителя
    if let winner_player_id = stmt
        .query_row(params![], |row| {
            Ok(
                row.get::<usize, i64>(0).unwrap_or(0), // player_id
            )
        })
        .optional()
        .expect("Failed to query winner data")
        .unwrap_or_default()
    {
        // Обрабатываем результаты запроса и сохраняем в бд
        println!("Player ID победителя: {}", winner_player_id);

        // Запрос к таблице players
        let mut stmt = conn
            .prepare("SELECT player_win_games FROM players WHERE player_id = ?")
            .expect("не удалось выбрать player_id в rec_result_table_players");

        // Получаем количество выигранных игр у победителя
        let win_games_count: i64 = stmt
            .query_row(params![winner_player_id], |row| Ok(row.get(0).unwrap_or(0)))
            .optional()
            .expect("Failed to query player_win_games")
            .unwrap_or_default();

        // Увеличиваем количество выигранных игр на 1
        let updated_win_games_count = win_games_count + 1;

        conn.execute(
            "UPDATE players SET
                                player_win_games = ?
                                WHERE player_id = ?",
            params![updated_win_games_count, winner_player_id,],
        )
        .expect("не удалось обновить данные после игры rec_result_table_players");
    } else {
        // Обработка ситуации, когда победитель не найден
        eprintln!("No winner found");
    }

    println!("окончание update_statistic_players");
}

//удаление таблиц reg_game_{} и schema_game_{} по окончании игры
pub async fn del_reg_and_schema_game(game_id: i64) {
    println!("запуск del_reg_and_schema_game");

    let conn = establish_connection();

    conn.execute(
        &format!("DROP TABLE IF EXISTS reg_game_{}", game_id),
        params![],
    )
    .expect(&format!("не удалось удалить таблицу reg_game_{}", game_id));

    conn.execute(
        &format!("DROP TABLE IF EXISTS schema_game_{}", game_id),
        params![],
    )
    .expect(&format!(
        "не удалось удалить таблицу schema_game_{}",
        game_id
    ));

    println!("таблица регистрации и схема игры {} удалены", game_id);
    println!("окончание del_reg_and_schema_game");
}

//записываем реальные данные игрока в промежуточную таблицу
pub async fn rec_pre_reg_player(
    player_id: i64,
    player_real_first_name: String,
    player_real_patronymic: String,
    player_real_last_name: String,
    player_real_location: String,
    player_real_phone_number: i64,
) {
    let mut conn = establish_connection();

    //начинаем транзакцию
    let transaction = conn.transaction().expect("Failed to start transaction");

    //выполняем запрос внутри транзакции
    let result = transaction
        .execute(
            "INSERT INTO pre_registrations_player (player_id, \
        player_real_first_name, player_real_patronymic, \
        player_real_last_name, player_real_location, \
        player_real_phone_number) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                player_id,
                player_real_first_name,
                player_real_patronymic,
                player_real_last_name,
                player_real_location,
                player_real_phone_number
            ],
        )
        .expect("не удалось записать промежуточные данные в rec_pre_reg_player");

    //проверяем результат выполнения запроса
    if result > 0 {
        //если все успешно - фиксируем транзакцию
        transaction
            .commit()
            .expect("Failed to commit transaction rec_pre_reg_player");
    } else {
        //в случае ошибка отменяем транзакцию
        transaction
            .rollback()
            .expect("Failed to rollback transaction rec_pre_reg_player");
        eprintln!(
            "Error inserting data for player_id {}: {} in rec_pre_reg_player",
            player_id, result
        );
    }
}

//получение данных из промежуточной таблицы перед записью в таблицу players
fn read_data_player_pre_reg(conn: &Connection, player_id: i64) -> RealPlayerData {
    let mut stmt = conn
        .prepare(
            "SELECT player_real_first_name, \
        player_real_patronymic, player_real_last_name, \
        player_real_location, player_real_phone_number \
        FROM pre_registrations_player WHERE player_id = ?\
        ORDER BY id DESC LIMIT 1",
        )
        .expect("ошибка получения данных в read_data_player_pre_reg");

    let real_player_data: RealPlayerData = stmt
        .query_row(params![player_id], |row| {
            Ok(RealPlayerData {
                player_real_first_name: row.get(0).unwrap_or_else(|_| "Имя не указано".to_string()),
                player_real_patronymic: row
                    .get(1)
                    .unwrap_or_else(|_| "Отчество не указано".to_string()),
                player_real_last_name: row
                    .get(2)
                    .unwrap_or_else(|_| "Фамилия не указана".to_string()),
                player_real_location: row.get(3).unwrap_or_else(|_| "Город не указан".to_string()),
                player_real_phone_number: row.get(4).unwrap_or(0),
            })
        })
        .unwrap_or_else(|e| {
            panic!(
                "ошибка при выполнении запроса в read_data_player_pre_reg: {}",
                e
            )
        });

    real_player_data
}

//переносим данные игрока из промежуточной таблицы в таблицу player
pub async fn rec_real_player_data_to_db(player_id: i64) {
    let mut conn = establish_connection();

    let real_player_data = read_data_player_pre_reg(&conn, player_id);

    //начинаем транзакцию
    let transaction_update = conn
        .transaction()
        .expect("Failed to start transaction_update");

    let result_update = transaction_update
        .execute(
            "UPDATE players SET
            player_real_first_name = ?1,
            player_real_patronymic = ?2,
            player_real_last_name = ?3,
            player_real_location = ?4,
            player_real_phone_number = ?5
        WHERE player_id = ?6",
            params![
                real_player_data.player_real_first_name,
                real_player_data.player_real_patronymic,
                real_player_data.player_real_last_name,
                real_player_data.player_real_location,
                real_player_data.player_real_phone_number,
                player_id,
            ],
        )
        .expect("не удалось вставить данные в rec_real_player_data_to_db");

    //проверяем результат выполнения запроса
    if result_update > 0 {
        //если все успешно - фиксируем транзакцию
        transaction_update
            .commit()
            .expect("Failed to commit transaction_update");
    } else {
        //в случае ошибка отменяем транзакцию
        transaction_update
            .rollback()
            .expect("Failed to rollback transaction_update");
        eprintln!(
            "Error inserting data for player_id {}: {} in rec_real_player_data_to_db",
            player_id, result_update
        );
    }

    let transaction_delete = conn
        .transaction()
        .expect("Failed to start transaction_delete");

    let result_delete = transaction_delete
        .execute(
            "DELETE FROM pre_registrations_player WHERE player_id = ?",
            params![player_id],
        )
        .expect("не удалось удалить промежуточные данные rec_real_player_data_to_db");

    if result_delete > 0 {
        //если все успешно - фиксируем транзакцию
        transaction_delete
            .commit()
            .expect("Failed to commit transaction_delete");
    } else {
        //в случае ошибка отменяем транзакцию
        transaction_delete
            .rollback()
            .expect("Failed to rollback transaction_delete");
        eprintln!(
            "Error inserting data for player_id {}: {} in rec_real_player_data_to_db",
            player_id, result_delete
        );
    }
}

//структура результатов сыгранной игры
pub struct PlayerResultGame {
    pub player_id: i64,
    pub positive_count: i32,
    pub negative_count: i32,
    pub sum_score: i32,
}

//получение результатов сыгранной игры из таблицы game_{}
pub fn getting_game_results(game_id: i64) {
    println!("запуск getting_game_results");

    let mut results = Vec::new();

    let conn = establish_connection();

    //Выбор всех player_id из таблицы reg_game_{}
    let mut stmt = conn
        .prepare(&format!(
            "SELECT player_id FROM reg_game_{} WHERE player_id IS NOT NULL",
            game_id
        ))
        .expect(&format!(
            "не удалось выбрать игроков в reg_game_{} в getting_game_results",
            game_id
        ));

    let player_id_iter = stmt
        .query_map(params![], |row| row.get::<usize, i64>(0))
        .expect("не удалось выполнить sql-запрос в getting_game_results");

    // Шаг 2: Для каждого player_id определяем статистику по сыгранной игре из таблицы game_{game_id}
    for player_id in player_id_iter {
        match player_id {
            Ok(id) => {
                // Запрос для подсчета количества значений > 0 в столбце score
                let mut stmt_game_positive = conn
                    .prepare(&format!(
                        "SELECT COUNT(*) FROM game_{} WHERE player_id = ? AND score > 0",
                        game_id
                    ))
                    .expect(&format!(
                        "Не удалось подготовить запрос для game_{}",
                        game_id
                    ));

                // Выполнение запроса и обработка результата
                let positive_count: Result<i32> =
                    stmt_game_positive.query_row([id], |row| row.get(0));
                let positive_count = match positive_count {
                    Ok(positive_score_count) => {
                        println!(
                            "Player ID: {}, Positive Score Count: {}",
                            id, positive_score_count
                        );
                        positive_score_count
                    }
                    Err(err) => {
                        eprintln!(
                                "Ошибка подсчёта количества положительных баллов player_id {}: {:?} в getting_game_results",
                                id, err
                            );
                        0 // Значение по умолчанию в случае ошибки
                    }
                };

                // Запрос для подсчета количества значений < 0 в столбце score
                let mut stmt_game_negative = conn
                    .prepare(&format!(
                        "SELECT COUNT(*) FROM game_{} WHERE player_id = ? AND score < 0",
                        game_id
                    ))
                    .expect(&format!(
                        "Не удалось подготовить запрос для game_{}",
                        game_id
                    ));

                // Выполнение запроса и обработка результата
                let negative_count: Result<i32> =
                    stmt_game_negative.query_row([id], |row| row.get(0));
                let negative_count = match negative_count {
                    Ok(negative_score_count) => {
                        println!(
                            "Player ID: {}, Negative Score Count: {}",
                            id, negative_score_count
                        );
                        negative_score_count
                    }
                    Err(err) => {
                        eprintln!(
                            "Ошибка подсчёта количества отрицательных баллов player_id {}: {:?} в getting_game_results",
                            id, err
                        );
                        0
                    }
                };

                // Запрос для подсчета суммы значений в столбце score
                let mut stmt_game_sum = conn
                    .prepare(&format!(
                        "SELECT SUM(score) FROM game_{} WHERE player_id = ?",
                        game_id
                    ))
                    .expect(&format!(
                        "Не удалось подготовить запрос для game_{}",
                        game_id
                    ));

                // Выполнение запроса и обработка результата
                let sum_score: Result<i32> = stmt_game_sum.query_row([id], |row| row.get(0));
                let sum_score = match sum_score {
                    Ok(sum) => {
                        println!("Player ID: {}, Sum of Scores: {}", id, sum);
                        sum
                    }
                    Err(err) => {
                        eprintln!(
                            "Ошибка подсчёта суммы баллов player_id {}: {:?} в getting_game_results",
                            id, err
                        );
                        0
                    }
                };

                // Создаем структуру PlayerResultGame с результатами и добавляем в вектор
                results.push(PlayerResultGame {
                    player_id: id,
                    positive_count,
                    negative_count,
                    sum_score,
                });
            }
            Err(err) => {
                eprintln!(
                    "Ошибка при выборке player ID: {:?} в getting_game_results",
                    err
                );
            }
        }
    }

    // Вызываем sending_game_results передавая в нее вектор результатов
    let game_results = results;

    // Запускаем асинхронную задачу, не создавая дополнительный runtime
    tokio::task::spawn(async move {
        if let Err(err) = sending_game_results(game_results).await {
            eprintln!("Ошибка при отправке результатов игры: {:?}", err);
        }
    });

    println!("окончание getting_game_results");
}

#[derive(Debug, Deserialize)]
pub struct TopicData {
    pub name: String,
}

#[post("/rec_schema_questions/<questions_pac_id>", data = "<data>")]
pub async fn rec_schema_questions(
    cookies: &CookieJar<'_>,
    questions_pac_id: i64,
    data: Json<Vec<TopicData>>,
) {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            println!(
                "запись схемы вопросов пакета {} для организатора {}",
                questions_pac_id, user_id
            );

            let conn = establish_connection();

            // Создаем таблицу и вставляем пустые строки вне транзакции
            conn.execute(
                &format!(
                    "CREATE TABLE IF NOT EXISTS schema_questions_{}_{} (
                        id INTEGER PRIMARY KEY,
                        question_id INTEGER
                    )",
                    user_id, questions_pac_id
                ),
                params![],
            )
            .expect("создание таблицы schema_questions не удалось в rec_schema_questions");

            conn.execute(
                &format!(
                    "INSERT INTO schema_questions_{}_{} (question_id) VALUES {}",
                    user_id,
                    questions_pac_id,
                    (0..270).map(|_| "(NULL)").collect::<Vec<&str>>().join(", ")
                ),
                params![],
            )
            .expect("вставка пустых строк не удалась в rec_schema_questions");

            // Создаем вектор, в который будем собирать все id перед записью
            let mut all_topic_ids: Vec<i32> = Vec::new();

            // Получаем темы пятёрки вопросов с веб-страницы
            for topic_data in data.iter() {
                let topic_name = &topic_data.name;

                let topic_ids: Vec<i32> = conn
                    .prepare(
                        &format!(
                            "SELECT id FROM questions_pac_{} WHERE topic_five_questions = ? ORDER BY price_question",
                            questions_pac_id
                        ),
                    )
                    .expect("подготовка запроса не удалась в rec_schema_questions")
                    .query_map(params![topic_name], |row| row.get(0))
                    .expect("не удалось получить id темы пятёрки вопросов в rec_schema_questions")
                    .filter_map(Result::ok)
                    .collect();

                // Добавляем все id из текущей темы в общий вектор
                all_topic_ids.extend(topic_ids);
            }

            // Переносим подготовку запроса за пределы цикла
            let mut stmt = conn
                .prepare(&format!(
                    "UPDATE schema_questions_{}_{} SET question_id = ? WHERE id = ?",
                    user_id, questions_pac_id
                ))
                .expect("подготовка запроса на обновление не удалась в rec_schema_questions");

            // Обновляем все строки в базе данных одним запросом
            for (index, topic_id) in all_topic_ids.iter().enumerate() {
                println!("запись");
                println!("{:?}", topic_id);
                stmt.execute(params![topic_id, index + 1])
                    .expect("вставка question_id в rec_schema_questions не удалась");
            }
        }
        _ => {}
    }
}

//функция получения города игрока
pub fn get_organiser_city(user_id: i64) -> String {
    let conn = establish_connection();
    let mut stmt = conn
        .prepare("SELECT city FROM users WHERE id = ?")
        .expect("не удалось выбрать город организатора");
    let city: Option<String> = stmt
        .query_row(params![user_id], |row| row.get(0))
        .optional()
        .expect("город организатора не найден");

    city.unwrap_or_else(|| "Город не указан".to_string())
}

//функция получения названия пакета вопросов
pub fn get_package_name(questions_pac_id: i64) -> String {
    let conn = establish_connection();
    let mut stmt = conn
        .prepare("SELECT package_name FROM register_questions_pac WHERE id = ?")
        .expect("не удалось выбрать пакет вопросов");
    let package_name: Option<String> = stmt
        .query_row(params![questions_pac_id], |row| row.get(0))
        .optional()
        .expect("пакет вопросов не найден");

    package_name.unwrap_or_else(|| "Название пакета вопросов не указано".to_string())
}

//функция получения названия темы пятёрки вопросов
pub fn get_topic_five_questions(questions_pac_id: i64, question_id: i32) -> String {
    let conn = establish_connection();
    let topic_five_questions: Option<String> = conn
        .prepare(&format!(
            "SELECT topic_five_questions FROM questions_pac_{} WHERE id = ?",
            questions_pac_id
        ))
        .and_then(|mut stmt| stmt.query_row(params![question_id], |row| row.get(0)))
        .optional()
        .expect("не удалось получить topic_five_questions");

    topic_five_questions.unwrap_or_else(|| "Название темы пятёрки вопросов не указано".to_string())
}

//функция получения названия последней вставленной темы пятёрки вопросов
pub fn get_topic_five_questions_last_insert(questions_pac_id: i64) -> String {
    let conn = establish_connection();
    let topic_five_questions: Option<String> = conn
        .query_row(
            &format!(
                "SELECT topic_five_questions FROM questions_pac_{} ORDER BY id DESC LIMIT 1",
                questions_pac_id
            ),
            [],
            |row| row.get(0),
        )
        .optional()
        .expect("не удалось получить topic_five_questions");

    topic_five_questions.unwrap_or_else(|| "Название темы пятёрки вопросов не указано".to_string())
}


struct NewPacName {
    user_id: i64,
    package_name: String,
}

//создание пакета вопросов
pub fn create_questions_pac(
    //создаем запись в таблице register_questions_pac и таблицу с id этой записи
    conn: &Connection,
    user_id: i64,
    package_name: String,
    game_type: String,
) -> std::result::Result<i64, Error> {

    // Проверка уникальности названия пакета вопросов
    let is_name_pac_unique: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM register_questions_pac WHERE package_name = ?",
            params![package_name],
            |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
        )
        .expect("не удалось выполнить запрос проверки уникальности");

    // Если вопрос не уникален, возвращаем ошибку
    if !is_name_pac_unique {
        return Err(Error::QueryReturnedNoRows.into());
    }

    conn.execute(
        "INSERT INTO register_questions_pac (user_id, package_name, stage, type) VALUES (?, ?, 0, ?)",
        params![user_id, package_name, game_type],
    )?;

    // Получаем id последней записи с конкретным user_id
    let last_inserted_id: i64 = conn
        .query_row(
            "SELECT id FROM register_questions_pac WHERE user_id = ? ORDER BY id DESC LIMIT 1",
            params![&user_id],
            |row| row.get(0),
        )
        .unwrap_or_default();

    // Создаем таблицу с использованием id из register_questions_pac
    let table_name = format!("questions_pac_{}", last_inserted_id);

    conn.execute(
        &format!(
            "CREATE TABLE IF NOT EXISTS {} (
                id INTEGER PRIMARY KEY,
                user_id INTEGER,
                question_pac_id INTEGER,
                topic_five_questions TEXT,
                question TEXT,
                answer TEXT,
                price_question INTEGER,
player_id INTEGER

            )",
            table_name
        ),
        [],
    )?;

    Ok(last_inserted_id)
}