use rocket::http::CookieJar; /* Импортируются структуры для работы с куки и
                             статусом HTTP из Rocket. */

use rocket_dyn_templates::Template; //Импорт для работы с шаблонами в Rocket.
use rusqlite::{params, OptionalExtension}; /* Импортируются функции и структуры из библиотеки
                                           rusqlite для взаимодействия с базой данных SQLite. */
use crate::db::{establish_connection, get_organiser_city, get_package_name};
use crate::web::users::{get_user_id_from_cookies, get_user_role, Context};
extern crate rand;
use rocket::serde::json::Json;
use rocket::serde::Serialize;
use rusqlite::Connection;

//переход на страницу передачи пакета вопросов другому организатору
#[post("/package_transfer")]
pub fn package_transfer(cookies: &CookieJar) -> Template {
    // Проверка, прошел ли пользователь аутентификацию
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let city = get_organiser_city(user_id);

                        let context = Context { header: city };
                        Template::render("transfer_questions_pac", &context)
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может подготавливать игры".to_string(),
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

//структура для получения организаторов в контекстное меню на сайте
#[derive(Serialize)]
pub struct OrganisersContextResponse {
    organisers_names: Vec<OrganisersPacContext>,
}

#[derive(Serialize)]
pub struct OrganisersPacContext {
    id: i64,
    user_name: String,
}

#[get("/organiser_data_context_menu")]
pub fn get_organiser_context_menu(cookies: &CookieJar) -> Json<OrganisersContextResponse> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            let connection = establish_connection();
            let organisers_names = query_organiser_context_menu(&connection, user_id);
            Json(OrganisersContextResponse { organisers_names })
        }
        Err(_) => Json(OrganisersContextResponse {
            organisers_names: vec![],
        }), // Вернуть объект с пустым массивом в случае ошибки
    }
}

// Функция для запроса организаторов для контекстного меню
fn query_organiser_context_menu(conn: &Connection, user_id: i64) -> Vec<OrganisersPacContext> {
    // Здесь выполните SQL-запрос, чтобы получить все данные о пакетах вопросов
    // и верните их в виде вектора структур QuestionsPacResponse
    let mut stmt = conn
        .prepare("SELECT id, username FROM users WHERE role = 'organiser' AND id != ?")
        .expect("Failed to prepare query");

    let organisers_names_iter = stmt
        .query_map(params![user_id], |row| {
            Ok(OrganisersPacContext {
                id: row.get(0)?,
                user_name: row.get(1)?,
            })
        })
        .expect("Failed to query organiser_context_menu");

    organisers_names_iter
        .map(|organisers_names_iter| organisers_names_iter.unwrap())
        .collect()
}

//функция обработки передачи пакета вопросов организатору выбранному из контекстного меню
#[post("/transfer_question_pac/<questions_pac_id>/<receiver_user_id>/<allow_transfer_value>")]
pub fn transfer_question_pac(
    cookies: &CookieJar,
    questions_pac_id: i64,
    receiver_user_id: i64,
    allow_transfer_value: i8,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let conn = establish_connection();

                        let package_name = get_package_name(questions_pac_id);

                        // Получаем организатора из базы данных
                        let mut stmt = conn
                            .prepare("SELECT username FROM users WHERE id = ?")
                            .expect("не удалось выбрать пакет вопросов");
                        let organiser_name: Option<String> = stmt
                            .query_row(params![receiver_user_id], |row| row.get(0))
                            .optional()
                            .expect("пакет вопросов не найден");

                        // Проверка отсутствия у получателя такого пакета.
                        let is_transfer_unique: bool = conn
                            .query_row(
                                "SELECT COUNT(*) FROM data_transfers WHERE (receiver_user_id = ? OR sender_user_id = ?) AND package_id = ?",
                                params![receiver_user_id, receiver_user_id, questions_pac_id],
                                |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
                            )
                            .expect("не удалось выполнить запрос проверки уникальности");

                        // Если такой пакет передавался от этого или этому организатору, прекращаем выполнение
                        if !is_transfer_unique {
                            let context = Context {
                                header: format!(
                                    "{} уже имеет пакет вопросов {}. \
                                Одним из организаторов (или Вами) этот пакет направлялся ранее.",
                                    organiser_name.unwrap_or_default(),
                                    package_name
                                ),
                            };
                            Err(Template::render("404", &context))
                        } else {
                            // Вставка данных в таблицу data_transfer
                            conn.execute(
                                "INSERT INTO data_transfers (\
                                sender_user_id,\
                                receiver_user_id,\
                                package_id, \
                                right_transfer_other) \
                                VALUES (?, ?, ?, ?)",
                                params![
                                    user_id,
                                    receiver_user_id,
                                    questions_pac_id,
                                    allow_transfer_value
                                ],
                            )
                            .expect("не удалось вставить данные в таблицу data_transfer");

                            let context = Context {
                                header: format!(
                                    "Пакет {} передан {}",
                                    package_name,
                                    organiser_name.unwrap_or_default()
                                ),
                            };

                            return Ok(Template::render("loggedin", &context));
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может подготавливать игры".to_string(),
                        };
                        Err(Template::render("index", &context))
                    }
                },
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
