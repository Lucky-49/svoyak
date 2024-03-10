use rocket::form::Form;
use rocket::http::CookieJar;
use rocket_dyn_templates::Template;
extern crate rand;
use crate::db::{create_questions_pac, establish_connection, get_organiser_city, get_package_name, get_topic_five_questions_last_insert};
use crate::web::users::{get_user_id_from_cookies, get_user_role, Context};
use rusqlite::{params, Connection, Error, OptionalExtension};

static QUANTITY_QUESTIONS: i32 = 270; //глобальная переменная общего количества вопросов в пакете, должна быть кратна 5 (в mod game количество раундов захардкодил)
static QUANTITY_TOPICS: i32 = QUANTITY_QUESTIONS / 5; //глобальная переменная общего количества тем пятёрок вопросов в пакете

#[derive(FromForm)] /* Атрибут derive с FromForm говорит Rocket, что структура
                    может быть автоматически создана из данных формы (HTML формы) при запросе HTTP. */
pub struct QuestionForm {
    //используется в fn rec_question_db для получения данных со страницы prepare_question
    pub topic_five_questions: String,
    pub question: String,
    pub answer: String,
    pub price_question: i32,
}

#[derive(Serialize)]
pub struct PackageNameTopicContext {
    pub header: String,
    pub header_pac_id: i64,
    pub header_pac_name: String,
    pub header_topic_five_questions: String,
}

#[post(
    "/rec_question/<questions_pac_id>",
    data = "<prepare_questions_topic_form>"
)]
pub fn rec_question_db(
    //запись организатором игры вопроса в бд
    cookies: &CookieJar, //принимаем ссылку на CookieJar
    prepare_questions_topic_form: Form<QuestionForm>, //принимаем данные формы типа QuestionForm с формы prepare_question.html.tera
    questions_pac_id: i64,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор
                        let player_id = None;
                        let player_question_id = None;
                        let input_data: QuestionForm = prepare_questions_topic_form.into_inner();
                        let conn = establish_connection();

                        match create_question(
                            /* Вызывает функцию create_question для создания нового
                            вопроса в базе данных. Результат этой операции обрабатывается в блоке match,
                            Если операция проходит успешно (Ok(_)), возвращается шаблон с сообщением об успешной
                            регистрации. */
                            &conn,
                            user_id,
                            questions_pac_id,
                            input_data.topic_five_questions,
                            input_data.question,
                            input_data.answer,
                            input_data.price_question,
                            player_id,
                            player_question_id,
                        ) {
                            Ok(_) => {
                                let city = get_organiser_city(user_id);

                                let package_name = get_package_name(questions_pac_id);

                                let topic_five_questions =
                                    get_topic_five_questions_last_insert(questions_pac_id);

                                let context = PackageNameTopicContext {
                                    header: city,
                                    header_pac_id: questions_pac_id,
                                    header_pac_name: package_name,
                                    header_topic_five_questions: topic_five_questions,
                                };
                                return Ok(Template::render("prepare_questions", &context));
                                // Изменено: успешная регистрация
                            }
                            Err(Error::QueryReturnedNoRows) => {
                                let context = Context {
                                    header: "Такой вопрос уже есть!".to_string(),
                                };
                                Err(Template::render("404", &context))
                            }
                            Err(err) => {
                                // Обработка других ошибок create_question, если они возникнут
                                let context = Context {
                                    header: format!("Ошибка при создании вопроса: {}", err),
                                };
                                Err(Template::render("404", &context))
                            }
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может создавать вопросы".to_string(),
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
            Err(Template::render("404", &context))
        }
    }
}

pub struct NewQuestion {
    //структура вопрос игры
    pub user_id: i64,
    pub questions_pac_id: i64,
    pub topic_five_questions: String,
    pub question: String,
    pub answer: String,
    pub price_question: i32,
    pub player_id: Option<i64>,
    pub player_question_id: Option<i64>,
}

pub fn create_question(
    conn: &Connection,
    user_id: i64,
    questions_pac_id: i64,
    topic_five_questions: String,
    question: String,
    answer: String,
    price_question: i32,
    player_id: Option<i64>,
    player_question_id: Option<i64>,
) -> Result<(), Error> {
    let new_question = NewQuestion {
        user_id,
        questions_pac_id,
        topic_five_questions,
        question,
        answer,
        price_question,
        player_id,
        player_question_id,
    };

    let table_name = format!("questions_pac_{}", new_question.questions_pac_id);

    // Проверка уникальности вопроса
    let is_question_unique: bool = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {} WHERE question = ?", table_name),
            params![&new_question.question],
            |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
        )
        .expect("не удалось выполнить запрос проверки уникальности");

    // Если вопрос не уникален, прекращаем выполнение
    if !is_question_unique {
        return Ok(());
    }

    // Проверка уникальности price_question для данной темы вопросов
    let is_price_question_unique: bool = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM {} WHERE topic_five_questions = ? AND price_question = ?",
                table_name
            ),
            params![
                &new_question.topic_five_questions,
                &new_question.price_question
            ],
            |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
        )
        .expect("не удалось выполнить запрос проверки уникальности price_question");

    // Если price_question не уникален для данной темы вопросов, прекращаем выполнение
    if !is_price_question_unique {
        return Ok(());
    }

    // Вставка данных в таблицу
    conn.execute(
        &format!(
            "INSERT INTO {} (user_id, question_pac_id, topic_five_questions,\
        question, answer, price_question, player_id) VALUES (?, ?, ?, ?, ?, ?, ?)",
            table_name
        ),
        params![
            &new_question.user_id,
            &new_question.questions_pac_id,
            &new_question.topic_five_questions,
            &new_question.question,
            &new_question.answer,
            &new_question.price_question,
            &new_question.player_id,
        ],
    )
    .expect("не удалось вставить данные в таблицу questions");

    //обновляем package_id вопроса игрока
    conn.execute(
        "UPDATE questions_players SET package_id = ? WHERE id = ?",
        params![questions_pac_id, player_question_id],
    )
    .expect("Вставка questions_pac_id в таблицу questions_players не удалась");

    Ok(())
}

#[post(
    "/rec_question_2/<questions_pac_id>",
    data = "<prepare_questions_form>"
)]
pub fn rec_question_db_2(
    //запись организатором игры вопроса в бд
    cookies: &CookieJar,                        //принимаем ссылку на CookieJar
    prepare_questions_form: Form<QuestionForm>, //принимаем данные формы типа QuestionForm с формы prepare_question.html.tera
    questions_pac_id: i64,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор

                        let player_id = None;

                        let player_question_id = None;

                        let input_data: QuestionForm = prepare_questions_form.into_inner();

                        let conn = establish_connection();

                        match create_question_2(
                            /* Вызывает функцию create_question для создания нового
                            вопроса в базе данных. Результат этой операции обрабатывается в блоке match,
                            Если операция проходит успешно (Ok(_)), возвращается шаблон с сообщением об успешной
                            регистрации. */
                            &conn,
                            user_id,
                            questions_pac_id.clone(),
                            input_data.topic_five_questions.clone(),
                            input_data.question,
                            input_data.answer,
                            input_data.price_question,
                            player_id,
                            player_question_id,
                        ) {
                            Ok(_) => {
                                // После успешного создания вопроса, проверяем количество записей с таким же topic_five_questions
                                let count_same_topic = conn
                        .query_row(
                            &format!(
                                "SELECT COUNT(*) FROM questions_pac_{} WHERE topic_five_questions = ?",
                                questions_pac_id
                            ),
                            params![&input_data.topic_five_questions],
                            |row| row.get(0),
                        )
                        .unwrap_or(0);

                                let number_questions =
                                    check_update_stage_pac(&conn, questions_pac_id);

                                if number_questions >= QUANTITY_QUESTIONS {
                                    let city = get_organiser_city(user_id);

                                    let package_name = get_package_name(questions_pac_id);

                                    let context = PackageNameContext {
                                        header: city,
                                        header_pac_id: questions_pac_id,
                                        header_pac_name: package_name,
                                    };
                                    return Ok(Template::render("questions_pac_done", &context));
                                } else {
                                    if count_same_topic >= 5 {
                                        // после пятого вопроса переходим на страницу prepare_questions_topic

                                        let city = get_organiser_city(user_id);

                                        let package_name = get_package_name(questions_pac_id);

                                        let context = PackageNameContext {
                                            header: city,
                                            header_pac_id: questions_pac_id,
                                            header_pac_name: package_name,
                                        };
                                        return Ok(Template::render(
                                            "prepare_questions_topic",
                                            &context,
                                        ));
                                    } else {
                                        let city = get_organiser_city(user_id);

                                        let package_name = get_package_name(questions_pac_id);

                                        let topic_five_questions =
                                            get_topic_five_questions_last_insert(questions_pac_id);

                                        let context = PackageNameTopicContext {
                                            header: city,
                                            header_pac_id: questions_pac_id,
                                            header_pac_name: package_name,
                                            header_topic_five_questions: topic_five_questions,
                                        };
                                        return Ok(Template::render("prepare_questions", &context));
                                        // Изменено: успешная регистрация
                                    }
                                }
                            }
                            Err(Error::QueryReturnedNoRows) => {
                                let context = Context {
                                    header: "Такой вопрос уже есть!".to_string(),
                                };
                                Err(Template::render("404", &context))
                            }
                            Err(err) => {
                                // Обработка других ошибок create_question, если они возникнут
                                let context = Context {
                                    header: format!("Ошибка при создании вопроса: {}", err),
                                };
                                Err(Template::render("404", &context))
                            }
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может создавать вопросы".to_string(),
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
            Err(Template::render("404", &context))
        }
    }
}

pub fn create_question_2(
    conn: &Connection,
    user_id: i64,
    questions_pac_id: i64,
    topic_five_questions: String,
    question: String,
    answer: String,
    price_question: i32,
    player_id: Option<i64>,
    player_question_id: Option<i64>,
) -> Result<(), Error> {
    let new_question = NewQuestion {
        user_id,
        questions_pac_id,
        topic_five_questions,
        question,
        answer,
        price_question,
        player_id,
        player_question_id,
    };

    let table_name = format!("questions_pac_{}", new_question.questions_pac_id);

    // Проверка уникальности вопроса
    let is_question_unique: bool = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {} WHERE question = ?", table_name),
            params![&new_question.question],
            |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
        )
        .expect("не удалось выполнить запрос проверки уникальности");

    // Если вопрос не уникален, прекращаем выполнение
    if !is_question_unique {
        return Ok(());
    }

    // Проверка уникальности price_question для данной темы вопросов
    let is_price_question_unique: bool = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM {} WHERE topic_five_questions = ? AND price_question = ?",
                table_name
            ),
            params![
                &new_question.topic_five_questions,
                &new_question.price_question
            ],
            |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
        )
        .expect("не удалось выполнить запрос проверки уникальности price_question");

    // Если price_question не уникален для данной темы вопросов, прекращаем выполнение
    if !is_price_question_unique {
        return Ok(());
    }

    conn.execute(
        &format!(
            "INSERT INTO {} (user_id, question_pac_id, topic_five_questions,\
        question, answer, price_question, player_id) VALUES (?, ?, ?, ?, ?, ?, ?)",
            table_name
        ),
        params![
            &new_question.user_id,
            &new_question.questions_pac_id,
            &new_question.topic_five_questions,
            &new_question.question,
            &new_question.answer,
            &new_question.price_question,
            &new_question.player_id,
        ],
    )
    .expect("не удалось вставить данные в таблицу questions");

    //обновляем package_id вопроса игрока
    conn.execute(
        "UPDATE questions_players SET package_id = ? WHERE id = ?",
        params![questions_pac_id, player_question_id],
    )
    .expect("Вставка questions_pac_id в таблицу questions_players не удалась");

    Ok(())
}

// функция проверки общего количества вопросов в пакете
fn check_update_stage_pac(conn: &Connection, questions_pac_id: i64) -> i32 {
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

    // Если количество записей не достигло (задано глобальной переменной), обновляем поле stage в register_questions_pac
    if count < QUANTITY_QUESTIONS {
        conn.execute(
            "UPDATE register_questions_pac SET stage = 0 WHERE id = ?",
            params![questions_pac_id],
        )
        .expect("ошибка изменения stage");
    }
    // Если количество записей достигло (задано глобальной переменной), обновляем поле stage в register_questions_pac
    else if count == QUANTITY_QUESTIONS {
        conn.execute(
            "UPDATE register_questions_pac SET stage = 1 WHERE id = ?",
            params![questions_pac_id],
        )
        .expect("ошибка изменения stage");
    };
    count
}

#[derive(FromForm)]
pub struct PackageNameForm {
    package_name_form: String,
    game_type: String,
}

#[derive(Serialize)]
pub struct PackageNameContext {
    pub header: String,
    pub header_pac_id: i64,
    pub header_pac_name: String,
}

#[post("/rec_questions_pac", data = "<prepare_questions_pac_form>")]
pub fn rec_questions_pac(
    cookies: &CookieJar,
    prepare_questions_pac_form: Form<PackageNameForm>,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор

                        let input_data: PackageNameForm = prepare_questions_pac_form.into_inner();

                        println!("пакет {}, тип игры {}", input_data.package_name_form,
                                 input_data.game_type);

                        let conn = establish_connection();

                        match create_questions_pac(&conn,
                                                   user_id,
                                                   input_data.package_name_form,
                                                   input_data.game_type) {
                            Ok(questions_pac_id) => {
                                let city = get_organiser_city(user_id);

                                let package_name = get_package_name(questions_pac_id);

                                let context = PackageNameContext {
                                    header: city,
                                    header_pac_id: questions_pac_id,
                                    header_pac_name: package_name,
                                };
                                return Ok(Template::render("prepare_questions_topic", &context));
                            }
                            Err(err) => {
                                // Обработка ошибок
                                if let Error::QueryReturnedNoRows = err {
                                    // Неуникальное имя пакета
                                    let context = Context {
                                        header: "Такое название пакета уже есть!".to_string(),
                                    };
                                    Err(Template::render("404", &context))
                                } else {
                                    // Обработка других ошибок
                                    let context = Context {
                                        header: format!("Ошибка при создании пакета: {}", err),
                                    };
                                    Err(Template::render("404", &context))
                                }
                            }
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может создавать пакеты".to_string(),
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
            Err(Template::render("404", &context))
        }
    }
}


#[derive(FromForm, Debug)]
pub struct PackageDataContextForm {
    package_name: String,
    topic_five_questions: String,
    question: String,
    answer: String,
    price_question: i32,
}
//запись вопроса игрока в новый пакет вопросов
#[post(
    "/rec_question_from_player_context/<player_question_id>",
    data = "<prepare_questions_pac_context_form>"
)]
pub fn rec_question_from_player_context(
    cookies: &CookieJar,
    player_question_id: Option<i64>,
    prepare_questions_pac_context_form: Form<PackageDataContextForm>,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор

                        let input_data: PackageDataContextForm =
                            prepare_questions_pac_context_form.into_inner();

                        let conn = establish_connection();

                        //получение player_id из questions_players
                        let mut stmt = conn
                            .prepare("SELECT player_id FROM questions_players WHERE id = ?")
                            .expect("не удалось выбрать игрока");
                        let player_id: Option<i64> = stmt
                            .query_row(params![player_question_id], |row| row.get(0))
                            .expect("player_id не найден");

                        // Проверка уникальности названия пакета вопросов
                        let is_name_pac_unique: bool = conn
                            .query_row(
                                "SELECT COUNT(*) FROM register_questions_pac WHERE package_name = ?",
                                params![&input_data.package_name],
                                |row| Ok(row.get::<usize, i64>(0) == Ok(0)),
                            )
                            .expect("не удалось выполнить запрос проверки уникальности");

                        let context = Context {
                            header: "Такое название пакета уже есть!".to_string(),
                        };

                        // Если вопрос не уникален, возвращаем ошибку
                        if !is_name_pac_unique {
                            return Err(Template::render("404", &context));
                        } else {
                            conn.execute(
                                "INSERT INTO register_questions_pac (user_id, package_name, stage) VALUES (?, ?, 0)",
                                params![&user_id, &input_data.package_name,],
                            )
                                .expect("не удалось создать запись rec_question_from_player_context");

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
                                         player_id INTEGER)",
                                    table_name
                                ),
                                [],
                            )
                            .expect("не удалось создать таблицу rec_question_from_player_context");

                            let questions_pac_id = last_inserted_id;

                            match create_question(
                                /* Вызывает функцию create_question для создания нового
                                вопроса в базе данных. Результат этой операции обрабатывается в блоке match,
                                Если операция проходит успешно (Ok(_)), возвращается шаблон с сообщением об успешной
                                регистрации. */
                                &conn,
                                user_id,
                                questions_pac_id,
                                input_data.topic_five_questions,
                                input_data.question,
                                input_data.answer,
                                input_data.price_question,
                                player_id,
                                player_question_id,
                            ) {
                                Ok(_) => {
                                    let city = get_organiser_city(user_id);

                                    let package_name = get_package_name(questions_pac_id);

                                    let topic_five_questions =
                                        get_topic_five_questions_last_insert(questions_pac_id);

                                    let context = PackageNameTopicContext {
                                        header: city,
                                        header_pac_id: questions_pac_id,
                                        header_pac_name: package_name,
                                        header_topic_five_questions: topic_five_questions,
                                    };

                                    return Ok(Template::render("prepare_questions", &context));
                                    // Изменено: успешная регистрация
                                }
                                Err(Error::QueryReturnedNoRows) => {
                                    let context = Context {
                                        header: "Такой вопрос уже есть!".to_string(),
                                    };
                                    Err(Template::render("404", &context))
                                }
                                Err(err) => {
                                    // Обработка других ошибок create_question, если они возникнут
                                    let context = Context {
                                        header: format!("Ошибка при создании вопроса: {}", err),
                                    };
                                    Err(Template::render("404", &context))
                                }
                            }
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может создавать пакеты".to_string(),
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
            Err(Template::render("404", &context))
        }
    }
}

#[derive(FromForm)]
pub struct EditQuestionForm {
    //используется в fn re_rec_question для получения данных со страницы edit_question
    question: String,
    answer: String,
    price_question: i32,
}
#[post(
    "/re_rec_question/<questions_pac_id>/<question_id>",
    data = "<edit_question_form>"
)]
pub fn re_rec_question_db(
    //запись организатором игры вопроса в бд
    cookies: &CookieJar,                        //принимаем ссылку на CookieJar
    edit_question_form: Form<EditQuestionForm>, //принимаем данные формы типа с формы edit_questions
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

                        let input_data: EditQuestionForm = edit_question_form.into_inner();

                        let conn = establish_connection();

                        match edit_question(
                            /* Вызывает функцию edit_question для перезаписи
                            вопроса в базе данных. Результат этой операции обрабатывается в блоке match,
                            Если операция проходит успешно (Ok(_)), возвращается шаблон с сообщением об успешной
                            регистрации. */
                            &conn,
                            questions_pac_id,
                            question_id,
                            input_data.question,
                            input_data.answer,
                            input_data.price_question,
                        ) {
                            Ok(_) => {
                                let city = get_organiser_city(user_id);

                                let package_name = get_package_name(questions_pac_id);

                                let context = PackageNameContext {
                                    header: city,
                                    header_pac_id: questions_pac_id,
                                    header_pac_name: package_name,
                                };

                                let mut stmt = conn
                                    .prepare(
                                        "SELECT stage FROM register_questions_pac WHERE id = ?",
                                    )
                                    .expect("не удалось выбрать пакет вопросов");

                                let stage: i64 = stmt
                                    .query_row(params![questions_pac_id], |row| row.get(0))
                                    .expect("пакет вопросов не найден");

                                if stage == 1 {
                                    return Ok(Template::render("questions_pac_done", &context));
                                }

                                return Ok(Template::render("edit_pac", &context));
                            }

                            Err(err) => {
                                // Обработка других ошибок edit_question, если они возникнут
                                let context = Context {
                                    header: format!("Ошибка при создании вопроса: {}", err),
                                };
                                Err(Template::render("404", &context))
                            }
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может редактировать вопросы".to_string(),
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
            Err(Template::render("404", &context))
        }
    }
}

#[derive(Serialize)]
struct EditQuestion {
    questions_pac_id: i64,
    question_id: i32,
    question: String,
    answer: String,
    price_question: i32,
}

pub fn edit_question(
    conn: &Connection,
    questions_pac_id: i64,
    question_id: i32,
    question: String,
    answer: String,
    price_question: i32,
) -> Result<(), Error> {
    let edit_question = EditQuestion {
        questions_pac_id,
        question_id,
        question,
        answer,
        price_question,
    };

    let table_name = format!("questions_pac_{}", edit_question.questions_pac_id);

    // Обновление данных в таблице
    conn.execute(
        &format!(
            "UPDATE {} SET question = ?, answer = ?, price_question = ? WHERE id = ?",
            table_name
        ),
        params![
            &edit_question.question,
            &edit_question.answer,
            &edit_question.price_question,
            &edit_question.question_id,
        ],
    )
    .expect("не удалось вставить данные в таблицу questions");
    Ok(())
}

#[derive(Serialize)]
struct AddQuestionTopic {
    header: String,
    header_pac_id: i64,
    header_pac_name: String,
    header_topic_five_questions: String,
}
#[post("/add_ques_in_topic/<questions_pac_id>/<topic_name>")]
pub fn add_ques_in_topic(
    cookies: &CookieJar,
    questions_pac_id: i64,
    topic_name: String,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let city = get_organiser_city(user_id);

                        let package_name = get_package_name(questions_pac_id);

                        let context = AddQuestionTopic {
                            header: city,
                            header_pac_id: questions_pac_id,
                            header_pac_name: package_name,
                            header_topic_five_questions: Some(topic_name).unwrap_or_else(|| {
                                "Название темы пятёрки вопросов не указано".to_string()
                            }),
                        };

                        return Ok(Template::render("prepare_questions", &context));
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

#[post("/add_topic/<questions_pac_id>")]
pub fn add_topic(cookies: &CookieJar, questions_pac_id: i64) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let conn = establish_connection();

                        // Перед созданием новой темы пятёрки вопросов, проверяем количество имеющихся в пакете тем пятёрок
                        let number_unique_topic = conn
                            .query_row(
                                &format!(
                                    "SELECT COUNT(DISTINCT topic_five_questions) FROM questions_pac_{}",
                                    questions_pac_id
                                ),
                                params![],
                                |row| row.get(0),
                            )
                            .unwrap_or(0);

                        //если в пакете создано максимально допустимое количество тем пятёрок вопросов, то
                        if number_unique_topic >= QUANTITY_TOPICS {
                            let context = Context {
                                header: "Создано максимальное количество тем пятёрок вопросов."
                                    .to_string(),
                            };
                            Err(Template::render("404", &context))
                        } else {
                            let city = get_organiser_city(user_id);

                            let package_name = get_package_name(questions_pac_id);

                            let context = PackageNameContext {
                                header: city,
                                header_pac_id: questions_pac_id,
                                header_pac_name: package_name,
                            };

                            return Ok(Template::render("prepare_questions_topic", &context));
                        }
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

#[derive(Serialize)]
struct PlayerQuestionsData {
    id: i64,
    player_topic_five_questions: Option<String>,
    player_question: String,
    player_answer_question: String,
}

#[derive(Serialize)]
struct PlayerQuestionsDataContext {
    header: String,
    player_question_id: i64,
    player_topic_five_questions: Option<String>,
    player_question: String,
    player_answer_question: String,
}

#[post("/create_questions_pac_context/<player_question_id>")]
pub fn create_questions_pac_context(
    cookies: &CookieJar,
    player_question_id: i64,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let conn = establish_connection();

                        let city = get_organiser_city(user_id);

                        // Получение данных вопроса который будет редактироваться для автозаполнения формы
                        let autofill_question: Option<PlayerQuestionsData> = conn
                            .prepare("SELECT id, player_topic_five_questions, player_question, player_answer_question FROM questions_players WHERE id = ?")
                            .and_then(|mut stmt| {
                                stmt.query_row(params![player_question_id], |row| {
                                    Ok(PlayerQuestionsData {
                                        id: row.get(0)?,
                                        player_topic_five_questions: row.get(1)?,
                                        player_question: row.get(2)?,
                                        player_answer_question: row.get(3)?,
                                    })
                                })
                            })
                            .optional()
                            .expect("не удалось выбрать тему пятёрки вопросов");

                        match autofill_question {
                            Some(autofill_question) => {
                                let context = PlayerQuestionsDataContext {
                                    header: city,
                                    player_question_id: autofill_question.id,
                                    player_topic_five_questions: autofill_question
                                        .player_topic_five_questions,
                                    player_question: autofill_question.player_question,
                                    player_answer_question: autofill_question
                                        .player_answer_question,
                                };

                                return Ok(Template::render(
                                    "prepare_questions_pac_context",
                                    &context,
                                ));
                            }
                            None => {
                                // Обработка случая, когда вопросы отсутствуют
                                let context = Context {
                                    header: "Отсутствуют вопросы для редактирования".to_string(),
                                };
                                return Ok(Template::render("404", &context));
                            }
                        }
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

//структура для автозаполнения страницы prepare_questions_context
#[derive(Serialize)]
struct PlayerQuestionsDataTopicContext {
    header: String,
    player_question_id: i64,
    header_pac_id: i64, //он же questions_pac_id
    package_name: Option<String>,
    player_topic_five_questions: Option<String>,
    player_question: String,
    player_answer_question: String,
}

//переход на страницу prepare_questions_context с автозаполнением формы
#[post("/add_in_topic_question_player/<questions_pac_id>/<player_topic_five_questions>/<player_question_id>")]
pub fn add_in_topic_question_player(
    cookies: &CookieJar,
    questions_pac_id: i64,
    player_topic_five_questions: Option<String>,
    player_question_id: i64,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let conn = establish_connection();

                        let city = get_organiser_city(user_id);

                        //передаем название пакета на веб форму
                        let mut stmt = conn
                            .prepare("SELECT package_name FROM register_questions_pac WHERE id = ? ORDER BY id DESC LIMIT 1")
                            .expect("не удалось выбрать пакет вопросов");
                        let package_name: Option<String> = stmt
                            .query_row(params![questions_pac_id], |row| row.get(0))
                            .optional()
                            .expect("пакет вопросов не найден");

                        // Получение данных вопроса который будет редактироваться для автозаполнения формы
                        let autofill_question: Option<PlayerQuestionsData> = conn
                            .prepare("SELECT id, player_topic_five_questions, player_question, player_answer_question FROM questions_players WHERE id = ?")
                            .and_then(|mut stmt| {
                                stmt.query_row(params![player_question_id], |row| {
                                    Ok(PlayerQuestionsData {
                                        id: row.get(0)?,
                                        player_topic_five_questions: row.get(1)?,
                                        player_question: row.get(2)?,
                                        player_answer_question: row.get(3)?,
                                    })
                                })
                            })
                            .optional()
                            .expect("не удалось выбрать тему пятёрки вопросов");

                        match autofill_question {
                            Some(autofill_question) => {
                                let context = PlayerQuestionsDataTopicContext {
                                    header: city,
                                    player_question_id: autofill_question.id,
                                    header_pac_id: questions_pac_id,
                                    package_name,
                                    player_topic_five_questions,
                                    player_question: autofill_question.player_question,
                                    player_answer_question: autofill_question
                                        .player_answer_question,
                                };

                                return Ok(Template::render("prepare_questions_context", &context));
                            }
                            None => {
                                // Обработка случая, когда вопросы отсутствуют
                                let context = Context {
                                    header: "Отсутствуют вопросы для редактирования".to_string(),
                                };
                                return Ok(Template::render("404", &context));
                            }
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может редактировать вопросы".to_string(),
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
            Err(Template::render("404", &context))
        }
    }
}

#[derive(FromForm)]
pub struct QuestionDataContextForm {
    topic_five_questions: String,
    question: String,
    answer: String,
    price_question: i32,
}

//запись вопроса игрока со страницы prepare_questions_context
#[post(
    "/rec_in_topic_question_player/<questions_pac_id>/<player_question_id>",
    data = "<prepare_questions_context_form>"
)]
pub fn rec_in_topic_question_player(
    cookies: &CookieJar,
    questions_pac_id: i64,
    player_question_id: Option<i64>,
    prepare_questions_context_form: Form<QuestionDataContextForm>,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор
                        let input_data = prepare_questions_context_form.into_inner();

                        let conn = establish_connection();

                        //получение player_id из questions_players
                        let mut stmt = conn
                            .prepare("SELECT player_id FROM questions_players WHERE id = ?")
                            .expect("не удалось выбрать игрока");
                        let player_id: Option<i64> = stmt
                            .query_row(params![player_question_id], |row| row.get(0))
                            .expect("player_id не найден");

                        match create_question_2(
                            /* Вызывает функцию create_question для создания нового
                            вопроса в базе данных. Результат этой операции обрабатывается в блоке match,
                            Если операция проходит успешно (Ok(_)), возвращается шаблон с сообщением об успешной
                            регистрации. */
                            &conn,
                            user_id,
                            questions_pac_id,
                            input_data.topic_five_questions.clone(),
                            input_data.question,
                            input_data.answer,
                            input_data.price_question,
                            player_id,
                            player_question_id,
                        ) {
                            Ok(_) => {
                                // После успешного создания вопроса, проверяем количество записей с таким же topic_five_questions
                                let count_same_topic = conn
                                    .query_row(
                                        &format!(
                                            "SELECT COUNT(*) FROM questions_pac_{} WHERE topic_five_questions = ?",
                                            questions_pac_id
                                        ),
                                        params![&input_data.topic_five_questions],
                                        |row| row.get(0),
                                    )
                                    .unwrap_or(0);

                                let number_questions =
                                    check_update_stage_pac(&conn, questions_pac_id);

                                if number_questions >= QUANTITY_QUESTIONS {
                                    //если в пакете количество вопросов (задается глобальной переменной) то

                                    let city = get_organiser_city(user_id);

                                    let package_name = get_package_name(questions_pac_id);

                                    let context = PackageNameContext {
                                        header: city,
                                        header_pac_id: questions_pac_id,
                                        header_pac_name: package_name,
                                    };
                                    return Ok(Template::render("questions_pac_done", &context));
                                    // Изменено: успешная регистрация
                                } else {
                                    if count_same_topic >= 5 {
                                        // после пятого вопроса переходим на страницу prepare_questions_topic

                                        let city = get_organiser_city(user_id);

                                        let package_name = get_package_name(questions_pac_id);

                                        let context = PackageNameContext {
                                            header: city,
                                            header_pac_id: questions_pac_id,
                                            header_pac_name: package_name,
                                        };
                                        return Ok(Template::render(
                                            "prepare_questions_topic",
                                            &context,
                                        ));
                                    } else {
                                        let city = get_organiser_city(user_id);

                                        let package_name = get_package_name(questions_pac_id);

                                        let topic_five_questions =
                                            get_topic_five_questions_last_insert(questions_pac_id);

                                        let context = PackageNameTopicContext {
                                            header: city,
                                            header_pac_id: questions_pac_id,
                                            header_pac_name: package_name,
                                            header_topic_five_questions: topic_five_questions,
                                        };
                                        return Ok(Template::render("prepare_questions", &context));
                                        // Изменено: успешная регистрация
                                    }
                                }
                            }
                            Err(Error::QueryReturnedNoRows) => {
                                let context = Context {
                                    header: "Такой вопрос уже есть!".to_string(),
                                };
                                Err(Template::render("404", &context))
                            }
                            Err(err) => {
                                // Обработка других ошибок create_question, если они возникнут
                                let context = Context {
                                    header: format!("Ошибка при создании вопроса: {}", err),
                                };
                                Err(Template::render("404", &context))
                            }
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может создавать вопросы".to_string(),
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
            Err(Template::render("404", &context))
        }
    }
}

//создание новой темы пятёрки вопросов из контекстного меню на основе вопроса игрока
#[post("/create_topic_five_questions_context/<questions_pac_id>/<player_question_id>")]
pub fn create_topic_five_questions_context(
    cookies: &CookieJar,
    questions_pac_id: i64,
    player_question_id: i64,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let conn = establish_connection();

                        // Перед созданием новой темы пятёрки вопросов, проверяем количество имеющихся в пакете тем пятёрок
                        let number_unique_topic = conn
                            .query_row(
                                &format!(
                                    "SELECT COUNT(DISTINCT topic_five_questions) FROM questions_pac_{}",
                                    questions_pac_id
                                ),
                                params![],
                                |row| row.get(0),
                            )
                            .unwrap_or(0);

                        //если в пакете создано максимально допустимое количество тем пятёрок вопросов, то
                        if number_unique_topic >= QUANTITY_TOPICS {
                            let context = Context {
                                header: "Создано максимальное количество тем пятёрок вопросов."
                                    .to_string(),
                            };
                            Err(Template::render("404", &context))
                        } else {
                            let city = get_organiser_city(user_id);

                            //передаем название пакета на веб форму
                            let mut stmt = conn
                                .prepare("SELECT package_name FROM register_questions_pac WHERE id = ? ORDER BY id DESC LIMIT 1")
                                .expect("не удалось выбрать пакет вопросов");
                            let package_name: Option<String> = stmt
                                .query_row(params![questions_pac_id], |row| row.get(0))
                                .optional()
                                .expect("пакет вопросов не найден");

                            // Получение данных вопроса который будет редактироваться для автозаполнения формы
                            let autofill_question: Option<PlayerQuestionsData> = conn
                                .prepare("SELECT id, player_topic_five_questions, player_question, player_answer_question FROM questions_players WHERE id = ?")
                                .and_then(|mut stmt| {
                                    stmt.query_row(params![player_question_id], |row| {
                                        Ok(PlayerQuestionsData {
                                            id: row.get(0)?,
                                            player_topic_five_questions: row.get(1)?,
                                            player_question: row.get(2)?,
                                            player_answer_question: row.get(3)?,
                                        })
                                    })
                                })
                                .optional()
                                .expect("не удалось выбрать тему пятёрки вопросов");

                            match autofill_question {
                                Some(autofill_question) => {
                                    let context = PlayerQuestionsDataTopicContext {
                                        header: city,
                                        player_question_id: autofill_question.id,
                                        header_pac_id: questions_pac_id,
                                        package_name,
                                        player_topic_five_questions: autofill_question
                                            .player_topic_five_questions,
                                        player_question: autofill_question.player_question,
                                        player_answer_question: autofill_question
                                            .player_answer_question,
                                    };

                                    return Ok(Template::render(
                                        "prepare_questions_topic_context",
                                        &context,
                                    ));
                                }

                                None => {
                                    // Обработка случая, когда вопросы отсутствуют
                                    let context = Context {
                                        header: "Отсутствуют вопросы для редактирования"
                                            .to_string(),
                                    };
                                    return Ok(Template::render("404", &context));
                                }
                            }
                        }
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

//запись вопроса при создании новой темы из контекста
#[post(
    "/rec_topic_question_player_context/<questions_pac_id>/<player_question_id>",
    data = "<prepare_questions_topic_context_form>"
)]
pub fn rec_topic_question_player_context(
    cookies: &CookieJar,
    questions_pac_id: i64,
    player_question_id: Option<i64>,
    prepare_questions_topic_context_form: Form<QuestionDataContextForm>,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let input_data = prepare_questions_topic_context_form.into_inner();

                        let conn = establish_connection();

                        //получение player_id из questions_players
                        let mut stmt = conn
                            .prepare("SELECT player_id FROM questions_players WHERE id = ?")
                            .expect("не удалось выбрать игрока");
                        let player_id: Option<i64> = stmt
                            .query_row(params![player_question_id], |row| row.get(0))
                            .expect("player_id не найден");

                        match create_question(
                            /* Вызывает функцию create_question для создания нового
                            вопроса в базе данных. Результат этой операции обрабатывается в блоке match,
                            Если операция проходит успешно (Ok(_)), возвращается шаблон с сообщением об успешной
                            регистрации. */
                            &conn,
                            user_id,
                            questions_pac_id,
                            input_data.topic_five_questions.clone(),
                            input_data.question,
                            input_data.answer,
                            input_data.price_question,
                            player_id,
                            player_question_id,
                        ) {
                            Ok(_) => {
                                let city = get_organiser_city(user_id);

                                let package_name = get_package_name(questions_pac_id);

                                let topic_five_questions =
                                    get_topic_five_questions_last_insert(questions_pac_id);

                                let context = PackageNameTopicContext {
                                    header: city,
                                    header_pac_id: questions_pac_id,
                                    header_pac_name: package_name,
                                    header_topic_five_questions: topic_five_questions,
                                };
                                return Ok(Template::render("prepare_questions", &context));
                                // Изменено: успешная регистрация
                            }
                            Err(Error::QueryReturnedNoRows) => {
                                let context = Context {
                                    header: "Такой вопрос уже есть!".to_string(),
                                };
                                Err(Template::render("404", &context))
                            }
                            Err(err) => {
                                // Обработка других ошибок create_question, если они возникнут
                                let context = Context {
                                    header: format!("Ошибка при создании вопроса: {}", err),
                                };
                                Err(Template::render("404", &context))
                            }
                        }
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может создавать вопросы".to_string(),
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
            Err(Template::render("404", &context))
        }
    }
}
