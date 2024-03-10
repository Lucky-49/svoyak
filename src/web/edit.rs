use crate::db::{
    establish_connection, get_organiser_city, get_package_name, get_topic_five_questions,
};
use rocket::http::CookieJar;
use rocket_dyn_templates::Template;
use rusqlite::{params, OptionalExtension};

extern crate rand;
use crate::web::rec_question::PackageNameContext;
use crate::web::users::{get_user_id_from_cookies, get_user_role, Context};

#[post("/view_pac/<questions_pac_id>")]
pub fn view_pac(cookies: &CookieJar, questions_pac_id: i64) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {

                        let conn = establish_connection();

                        conn.query_row(
                                &format!(
                                    "SELECT COUNT (*) FROM questions_pac_{}",
                                    questions_pac_id
                                ),
                                [],
                                |row| row.get(0),
                            )
                            .unwrap_or_else(|err| {
                                eprintln!(
                                    "Ошибка при проверке количества вопросов в функции edit_pac: {}",
                                    err
                                );
                                0 // Вернуть значение по умолчанию
                            });

                        let city = get_organiser_city(user_id);

                        let package_name = get_package_name(questions_pac_id);

                        let context = PackageNameContext {
                            header: city,
                            header_pac_id: questions_pac_id,
                            header_pac_name: package_name,
                        };

                        // Проверка, что пакет вопросов не передавался.
                        let is_transfer_unique: bool = conn
                            .query_row(
                                "SELECT COUNT(*) FROM data_transfers WHERE package_id = ?",
                                params![questions_pac_id],
                                |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
                            )
                            .expect("не удалось выполнить запрос проверки уникальности");

                        if !is_transfer_unique {
                            return Ok(Template::render("questions_pac_done_transfer", &context));
                        } else {
                            return Ok(Template::render("questions_pac_done", &context));
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может просматривать пакеты".to_string(),
                        };
                        Err(Template::render("404", &context))
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваша роль не определена".to_string(),
                    };
                    Err(Template::render("404", &context))
                }
            }
        }
        Err(_) => {
            // Обработка ошибок get_user_id_from_cookies
            let context = Context {
                header: "Ошибка при получении идентификатора пользователя".to_string(),
            };
            Ok(Template::render("404", &context))
        }
    }
}

#[post("/edit_pac/<questions_pac_id>")]
pub fn edit_pac(cookies: &CookieJar, questions_pac_id: i64) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор

                        let conn = establish_connection();
                        let count = conn
                            .query_row(
                                &format!(
                                    "SELECT COUNT (*) FROM questions_pac_{}",
                                    questions_pac_id
                                ),
                                [],
                                |row| row.get(0),
                            )
                            .unwrap_or_else(|err| {
                                eprintln!(
                        "Ошибка при проверке количества вопросов в функции edit_pac: {}",
                        err
                    );
                                0 // Вернуть значение по умолчанию
                            });

                        let city = get_organiser_city(user_id);

                        let package_name = get_package_name(questions_pac_id);

                        let context = PackageNameContext {
                            header: city,
                            header_pac_id: questions_pac_id,
                            header_pac_name: package_name,
                        };
                        if count == 0 {
                            return Ok(Template::render("prepare_questions_topic", &context));
                        } else {
                            return Ok(Template::render("edit_pac", &context));
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может просматривать пакеты".to_string(),
                        };
                        Err(Template::render("404", &context))
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваша роль не определена".to_string(),
                    };
                    Err(Template::render("404", &context))
                }
            }
        }
        Err(_) => {
            // Обработка ошибок get_user_id_from_cookies
            let context = Context {
                header: "Ошибка при получении идентификатора пользователя".to_string(),
            };
            Ok(Template::render("404", &context))
        }
    }
}

#[derive(Serialize)]
pub struct AutofillQuestion {
    header: String,
    header_pac_id: i64,
    header_pac_name: String,
    header_topic_five_questions: String,
    header_question_id: i32,
    question: String,
    answer: String,
    price_question: i32,
}

pub struct AutofillQuestionDB {
    question: String,
    answer: String,
    price_question: i32,
}

#[post("/edit_pac_que/<questions_pac_id>/<question_id>")]
pub fn edit_pac_que(
    cookies: &CookieJar,
    questions_pac_id: i64,
    question_id: i32,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор

                        let conn = establish_connection();

                        // Получение данных вопроса который будет редактироваться для автозаполнения формы
                        let autofill_question: Option<AutofillQuestionDB> = conn
                .prepare(&format!("SELECT question, answer, price_question FROM questions_pac_{} WHERE id = ? LIMIT 1", questions_pac_id))
                .and_then(|mut stmt| {
                    stmt.query_row(params![question_id], |row| {
                        Ok(AutofillQuestionDB {
                            // здесь указывайте поля вашей структуры и их типы
                            question: row.get(0).unwrap_or_default(), // если первое поле - текст вопроса
                            answer: row.get(1).unwrap_or_default(),   // а второе - текст ответа
                            price_question: row.get(2).unwrap_or_default(), // а третье - цена вопроса
                        })
                    })
                })
                .optional()
                .expect("не удалось получить первый вопрос");

                        let city = get_organiser_city(user_id);

                        let package_name = get_package_name(questions_pac_id);

                        let topic_five_questions =
                            get_topic_five_questions(questions_pac_id, question_id);

                        match autofill_question {
                            Some(autofill_question) => {
                                let context = AutofillQuestion {
                                    header: city,
                                    header_pac_id: questions_pac_id,
                                    header_pac_name: package_name,
                                    header_topic_five_questions: topic_five_questions,
                                    header_question_id: question_id,
                                    question: autofill_question.question,
                                    answer: autofill_question.answer,
                                    price_question: autofill_question.price_question,
                                };

                                return Ok(Template::render("edit_questions", &context));
                            }
                            None => {
                                // Обработка случая, когда вопросы отсутствуют
                                let context = Context {
                                    header: "Отсутствуют вопросы для редактирования".to_string(),
                                };
                                return Ok(Template::render("404", &context));
                            }
                        };
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может просматривать вопросы".to_string(),
                        };
                        Err(Template::render("404", &context))
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваша роль не определена".to_string(),
                    };
                    Err(Template::render("404", &context))
                }
            }
        }
        Err(_) => {
            // Обработка ошибок get_user_id_from_cookies
            let context = Context {
                header: "Ошибка при получении идентификатора пользователя".to_string(),
            };
            Ok(Template::render("404", &context))
        }
    }
}
