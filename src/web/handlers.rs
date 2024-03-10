/* В этом файле находиться функции, обрабатывающие HTTP-запросы и управляющие логикой
обработки запросов */

use bcrypt::{verify, DEFAULT_COST};
use chrono::NaiveDate;

use csrf::{AesGcmCsrfProtection, CsrfProtection}; /* Импортируются функции из библиотеки csrf для
                                                  защиты от межсайтовых подделок запросов (CSRF). */
use data_encoding::BASE64; /* Импортируется константа BASE64 из библиотеки data_encoding для
                          работы с кодированием Base64. */
use rocket::form::Form;
/* Импортируется структура Form из крана Rocket, которая используется для
обработки данных форм в HTTP-запросах. */

use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect; /* Импортируются структуры для работы с куки и
                                статусом HTTP из Rocket. */
use rocket_dyn_templates::Template; //Импорт для работы с шаблонами в Rocket.
use rusqlite::params;
/* Импортируются функции и структуры из библиотеки
rusqlite для взаимодействия с базой данных SQLite. */
use crate::db::{create_game_table, establish_connection, get_organiser_city, get_package_name};
use crate::tg::bot::announce_game_bot;
use crate::tg::token::SECRET_KEY_CSFR;
use crate::web::data_form::table_exists_schema_questions;
use crate::web::rec_question::PackageNameContext;
use crate::web::users::{
    create_new_user, create_new_user_session, generate_session_token, get_id_users_table,
    get_password_hash_from_username, get_user_id_from_cookies, get_user_role, CSRFContext, Context,
    UserFormLogin, UserFormSignup,
};
use time::Duration;

//Импортируется Duration из библиотеки time для работы с временными интервалами.
extern crate rand; /* внешняя зависимость от библиотеки rand, которая предоставляет генераторы
                   случайных чисел. */

//загрузка стартовой страницы сайта
#[get("/")]
pub fn index() -> Template {
    /*  let key_length = 32; //генерация секретного ключа
    let mut rng = rand::thread_rng();

    let key: String = (0..key_length)
        .map(|_| rng.gen_range(33..127) as u8 as char)  // Диапазон ASCII символов (33-126)
        .collect();


    println!("{:?}", key); */

    let context = Context {
        //обязательная переменная без которой не будет рендера страницы
        header: "Стартовая страница".to_string(),
    };
    Template::render("index", &context) //название файла стартовой страницы
}

/* выход из сервиса */
#[post("/logout")]
pub fn logout(cookies: &CookieJar) -> Template {
    cookies.remove_private(Cookie::from("session_token"));

    let context = Context {
        header: "Вы успешно вышли из системы".to_string(),
    };
    Template::render("index", &context)
}

//функция перехода на страницу login
//функция обрабатывает действия на стартовой странице сайта
#[get("/log_in_page")]
pub fn go_to_log_in_page(cookies: &CookieJar) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(_user_id) => {
            let context = Context {
                header: "Вход для организаторов".to_string(),
            };
            Template::render("loggedin", &context) //название файла страницы куда будет перенаправлен юзер после аутентификации
        }
        Err(_not_logged_in) => {
            // Генерация CSRF-токена для приватной куки и его установка
            let protect = AesGcmCsrfProtection::from_key(SECRET_KEY_CSFR);
            let (token_csrf, cookie) = protect
                .generate_token_pair(None, 300)
                .expect("не удалось сгенерировать пару токен/куки");
            let mut c = Cookie::new("csrf", cookie.b64_string());
            c.set_max_age(Duration::hours(24));
            cookies.add_private(c);

            let context = CSRFContext {
                header: "Вход для организаторов".to_string(), //текст, который будет выведен на страницу
                csrf: token_csrf.b64_string(),
            };
            Template::render("login", &context) //название файла стартовой страницы
        }
    }
}

// прохождение авторизации
#[post("/log_in", data = "<user_login_form>")]
pub fn login(
    cookies: &CookieJar,
    user_login_form: Form<UserFormLogin>,
) -> Result<Template, Template> {
    match cookies.get_private("csrf") {
        Some(cookie) => {
            let input_data: UserFormLogin = user_login_form.into_inner();
            let protect = AesGcmCsrfProtection::from_key(SECRET_KEY_CSFR);
            let token_csrf_bytes = BASE64
                .decode(input_data.csrf.as_bytes())
                .expect("token_csrf not BASE64");
            let cookie_byte = BASE64
                .decode(cookie.value().to_string().as_bytes())
                .expect("cookie not BASE64");

            let parsed_token_csrf = protect
                .parse_token(&token_csrf_bytes)
                .expect("token_csrf not parsed");

            let parsed_cookie = protect
                .parse_cookie(&cookie_byte)
                .expect("cookie not parsed");

            if protect.verify_token_pair(&parsed_token_csrf, &parsed_cookie) {
                match get_password_hash_from_username(input_data.username_login.clone()) {
                    Ok(password_hash) => match verify(&input_data.password_login, &password_hash) {
                        Ok(password_match) => {
                            if password_match {
                                match generate_session_token(64) {
                                    Ok(session_token) => {
                                        let connection = establish_connection();
                                        let user_id = get_id_users_table(input_data.username_login);
                                        let _ = create_new_user_session(
                                            &connection,
                                            user_id,
                                            session_token.clone(),
                                        );
                                        let mut cookie =
                                            Cookie::new("session_token", session_token);

                                        cookie.set_max_age(Duration::hours(3)); //устанавливаем время действия cookie
                                        cookies.add_private(cookie);

                                        match get_user_role(user_id) {
                                            //определение роли юзера
                                            Ok(role) => match role.as_str() {
                                                "visitor" => {
                                                    let context = Context {
                                                        header: "".to_string(),
                                                    };
                                                    Ok(Template::render("visitor", &context))
                                                }
                                                "organiser" => {
                                                    let context = Context {
                                                        header: "Вы вошли в систему".to_string(),
                                                    };
                                                    Ok(Template::render("loggedin", &context))
                                                }
                                                "admin" => {
                                                    let context = Context {
                                                        header: "".to_string(),
                                                    };
                                                    Ok(Template::render("admin", &context))
                                                }
                                                "superadmin" => {
                                                    let context = Context {
                                                        header: "".to_string(),
                                                    };
                                                    Ok(Template::render("superadmin", &context))
                                                }
                                                _ => {
                                                    let context = Context {
                                                        header: "роль не определена".to_string(),
                                                    };
                                                    return Err(Template::render("404", &context));
                                                }
                                            },
                                            Err(_) => {
                                                let context = Context {
                                                    header: "роль не определена".to_string(),
                                                };
                                                return Err(Template::render("404", &context));
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        let context = Context {
                                            header: "Login Failed!".to_string(),
                                        };
                                        return Err(Template::render("404", &context));
                                    }
                                }
                            } else {
                                let context = Context {
                                    header: "Password incorrect".to_string(),
                                };
                                return Err(Template::render("404", &context));
                            }
                        }
                        Err(_) => {
                            let context = Context {
                                header: "An error occurred".to_string(),
                            };
                            return Err(Template::render("404", &context));
                        }
                    },
                    Err(_) => {
                        let context = Context {
                            header: "Пользователь с таким логином не зарегистрирован".to_string(),
                        };
                        return Err(Template::render("404", &context));
                    }
                }
            } else {
                let context = Context {
                    header: "CSRF failed".to_string(),
                };
                return Err(Template::render("404", &context));
            }
        }
        None => {
            let context = Context {
                header: "Login failed".to_string(),
            };
            return Err(Template::render("404", &context));
        }
    }
}

/* Регистрация нового юзера */
#[post("/sign_up", data = "<user_signup_form>")] /* Этот атрибут указывает, что функция
                                                 register_user будет обрабатывать POST-запросы по пути "/sign_up" и ожидает данные формы с именем
                                                 "user_signup_form". */
pub fn register_user(
    // объявление функции register_user
    cookies: &CookieJar,                    //принимаем ссылку на CookieJar
    user_signup_form: Form<UserFormSignup>, //принимаем данные формы типа UserFormSignup с формы signup
) -> Result<Template, Template> {
    match cookies.get_private("csrf") {
        /* Здесь начинается блок match, который обрабатывает
        варианты, связанные с получением CSRF-токена из cookies. В зависимости от наличия токена
        выполняются соответствующие действия. */
        Some(cookie) => {
            /* Этот вариант срабатывает, если в cookies найден CSRF-токен,
            и он успешно извлечен (не является None). В этом случае переменная cookie содержит объект
            типа Cookie, представляющий CSRF-токен. */
            let input_data: UserFormSignup = user_signup_form.into_inner(); /* Здесь данные из
                                                                            формы user_signup_form преобразуются в объект типа UserFormSignup, представляющий данные,
                                                                            введенные пользователем при регистрации. */

            let protect = AesGcmCsrfProtection::from_key(SECRET_KEY_CSFR);
            let token_csrf_bytes = BASE64
                .decode(input_data.csrf.as_bytes()) /* CSRF-токен,
                полученный из формы, декодируется из BASE64 в байты. */
                .expect("token_csrf not BASE64");
            let cookie_byte = BASE64
                .decode(cookie.value().to_string().as_bytes()) /* Значение
                CSRF-токена из cookies также декодируется из BASE64 в байты. */
                .expect("cookie not BASE64");

            let parsed_token_csrf = protect
                .parse_token(&token_csrf_bytes)
                .expect("token_csrf not parsed"); /* CSRF-токен, представленный в виде байтов
                                                  (token_csrf_bytes), парсится (распаковывается) с использованием метода parse_token
                                                  объекта AesGcmCsrfProtection. Результат сохраняется в переменной parsed_token_csrf.
                                                  Если парсинг не удается (например, из-за некорректных данных), программа завершится
                                                  с сообщением об ошибке. */

            let parsed_cookie = protect
                .parse_cookie(&cookie_byte)
                .expect("cookie not parsed"); /* Значение cookie, представленное в виде байтов
                                              (cookie_byte), парсится с использованием метода parse_cookie объекта AesGcmCsrfProtection.
                                              Результат сохраняется в переменной parsed_cookie. Если парсинг не удается,
                                              программа завершится с сообщением об ошибке. */

            if protect.verify_token_pair(&parsed_token_csrf, &parsed_cookie) {
                /* Проверяет,
                прошла ли успешно проверка CSRF для пары токена и значения cookie. Если проверка проходит
                успешно, код внутри этого условия выполняется. */

                let user_id_result = get_user_id_from_cookies(cookies);

                match user_id_result {
                    Ok(user_id) => {
                        match get_user_role(user_id) {
                            //определение роли юзера
                            Ok(role) => match role.as_str() {
                                "admin" | "superadmin" => {
                                    //если админ или суперадмин

                                    let hashed_password = match bcrypt::hash(
                                        input_data.password_signup,
                                        DEFAULT_COST,
                                    ) {
                                        /* Хэширует введенный
                                        пользователем пароль с использованием библиотеки bcrypt. Если хеширование проходит успешно, хэш
                                        сохраняется в переменной hashed_password. */
                                        Ok(hashed) => hashed,

                                        Err(_) => {
                                            let context = Context {
                                                header: "Registration Failed!".to_string(),
                                            };
                                            return Err(Template::render("404", &context));
                                        }
                                    };

                                    let connection = establish_connection(); /* Устанавливает соединение с
                                                                             базой данных. Функция establish_connection возвращает объект, представляющий
                                                                             соединение с базой данных. */
                                    match create_new_user(
                                        /* Вызывает функцию create_new_user для создания нового
                                        пользователя в базе данных. Результат этой операции обрабатывается в блоке match,
                                        Если операция проходит успешно (Ok(_)), возвращается шаблон с сообщением об успешной
                                        регистрации. */
                                        &connection,
                                        input_data.username_signup,
                                        hashed_password,
                                        input_data.city_signup,
                                        input_data.role_signup,
                                        input_data.first_name_user,
                                        input_data.patronymic_user,
                                        input_data.last_name_user,
                                        input_data.phone_user,
                                        input_data.mail_user,
                                    ) {
                                        Ok(_) => {
                                            let context = Context {
                                                header: "Регистрация прошла успешно!".to_string(),
                                            };
                                            return Ok(Template::render("regdone", &context));
                                            // Изменено: успешная регистрация
                                        }
                                        Err(rusqlite::Error::QueryReturnedNoRows) => {
                                            let context = Context {
                                                header: "Логин уже занят!".to_string(),
                                            };
                                            return Err(Template::render("404", &context));
                                        }
                                        Err(_) => {
                                            let context = Context {
                                                header: "Registration Failed!".to_string(),
                                            };
                                            return Err(Template::render("404", &context));
                                            // Изменено: Ошибка входа
                                        }
                                    }
                                }
                                _ => {
                                    let context = Context {
                                        header: "роль не определена".to_string(),
                                    };
                                    return Err(Template::render("404", &context));
                                }
                            },
                            _ => {
                                let context = Context {
                                    header: "роль не определена".to_string(),
                                };
                                return Err(Template::render("404", &context));
                            }
                        }
                    }
                    Err(_) => {
                        let context = Context {
                            header: "Ошибка при получении user_id из cookies".to_string(),
                        };
                        Err(Template::render("404", &context))
                    }
                }
            } else {
                let context = Context {
                    header: "CSRF Verification Failed!".to_string(),
                };
                return Err(Template::render("404", &context)); // Изменено: CSRF не прошел
            }
        }
        None => {
            let context = Context {
                header: "Login Failed!".to_string(),
            };
            return Err(Template::render("404", &context)); // Изменено: Ошибка входа
        }
    }
}



//переход на страницу подготовки пакетов вопросов
#[post("/prepare_questions_pac")]
pub fn prepare_questions_pac(cookies: &CookieJar) -> Template {
    // Проверка, прошел ли пользователь аутентификацию
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {

                        let city = get_organiser_city(user_id);

                        let context = Context {
                            header: city
                        };

                        Template::render("prepare_questions_pac", &context)
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

/* переход на страницу подготовки вопроса */
#[post("/prepare_questions")]
pub fn prepare_questions(cookies: &CookieJar) -> Template {
    // Проверка, прошел ли пользователь аутентификацию
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            // Пользователь аутентифицирован, перейдите на страницу подготовки игры

            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор
                        let city = get_organiser_city(user_id);

                        let context = Context { header: city };

                        Template::render("prepare_questions", &context)
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может создавать вопросы".to_string(),
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

/* переход на страницу анонсирования игры announce_game */
#[get("/prepare_game")]
pub fn prepare_game(cookies: &CookieJar) -> Template {
    //кнопка объявить игру
    // Проверка, прошел ли пользователь аутентификацию
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            // Пользователь аутентифицирован, перейдите на страницу подготовки игры
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор
                        let city = get_organiser_city(user_id);

                        let context = Context { header: city };

                        Template::render("announce_game", &context)
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только организатор может объявлять игры".to_string(),
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

/* переход на страницу подготовки к игре */
#[get("/navigate_prepare_game")]
pub fn navigate_prepare_game(cookies: &CookieJar) -> Template {
    //кнопка объявить игру
    // Проверка, прошел ли пользователь аутентификацию
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            // Пользователь аутентифицирован, перейдите на страницу подготовки игры
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор
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

//функция перехода на страницу signup.html.tera
#[get("/sign_up_page")]
pub fn go_to_sign_up_page(cookies: &CookieJar) -> Template {
    // Генерация CSRF-токена для приватной куки и его установка
    let protect = AesGcmCsrfProtection::from_key(SECRET_KEY_CSFR);
    let (token_csrf, cookie) = protect
        .generate_token_pair(None, 300)
        .expect("не удалось сгенерировать пару токен/куки");
    let mut c = Cookie::new("csrf", cookie.b64_string());
    c.set_max_age(Duration::hours(24));
    cookies.add_private(c);

    let user_id_result = get_user_id_from_cookies(cookies);

    match user_id_result {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "superadmin" => {
                        //если организатор

                        let context = CSRFContext {
                            header: "Страница регистрации нового пользователя".to_string(), //текст, который будет выведен на страницу
                            csrf: token_csrf.b64_string(),
                        };
                        Template::render("signup", &context) //название файла стартовой страницы
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Только для superadmin".to_string(),
                        };
                        Template::render("index", &context)
                    }
                },
                _ => {
                    // Пользователь не аутентифицирован, перейдите на главную страницу
                    let context = Context {
                        header: "Ваша роль не определена".to_string(),
                    };
                    Template::render("index", &context)
                }
            }
        }
        _ => {
            // Пользователь не аутентифицирован, перейдите на главную страницу
            let context = Context {
                header: "Ваша ID не определен".to_string(),
            };
            Template::render("index", &context)
        }
    }
}

//переход на страницу questions_player
#[post("/questions_players")]
pub fn questions_players(cookies: &CookieJar) -> Template {
    // Проверка, прошел ли пользователь аутентификацию
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        //если организатор
                        // Пользователь аутентифицирован, перейдите на страницу questions_players
                        let city = get_organiser_city(user_id);

                        let context = Context { header: city };

                        Template::render("questions_players", &context)
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

#[get("/del_player_question/<id>")]
pub fn del_player_question(cookies: &CookieJar, id: i64) -> Template {
    // Проверка, прошел ли пользователь аутентификацию
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        del_question(id);
                        //если организатор
                        // Пользователь аутентифицирован, перейдите на страницу questions_players
                        let city = get_organiser_city(user_id);

                        let context = Context { header: city };

                        Template::render("questions_players", &context)
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

//удаление вопроса из таблицы вопросы от игроков
fn del_question(id: i64) {
    let conn = establish_connection();
    let sql_query = "DELETE FROM questions_players WHERE id = ?;";
    conn.execute(sql_query, params![id])
        .expect("Failed to execute delete query");
}

//объявление игры
#[derive(FromForm, Debug)]
pub struct AnnounceGame {
    datepicker: String,
    timepicker: String,
    announce_message: String,
    price_player: i32,
    price_spectator: i32,
    seats_spectator: i8,
}

#[post("/announce_game", data = "<announce_game_form>")]
pub async fn announce_game(
    cookies: &CookieJar<'_>,
    announce_game_form: Form<AnnounceGame>,
) -> Result<Template, Template> {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let announce_data = announce_game_form.into_inner();

                        let game_time = announce_data.timepicker;
                        let game_location = announce_data.announce_message;
                        let price_player = announce_data.price_player;
                        let price_spectator = announce_data.price_spectator;
                        let seats_spectator = announce_data.seats_spectator;

                        // Распарсить исходную дату
                        let parsed_data =
                            NaiveDate::parse_from_str(&*announce_data.datepicker, "%Y-%m-%d");

                        match parsed_data {
                            Ok(_data) => {
                                let game_day_parse =
                                    parsed_data.expect("REASON").format("%d.%m.%Y");

                                let game_day = game_day_parse.to_string();

                                let connection = establish_connection();

                                match create_game_table(
                                    &connection,
                                    user_id,
                                    game_day.clone(),
                                    game_time.clone(),
                                    game_location.clone(),
                                    price_player.clone(),
                                    price_spectator.clone(),
                                    seats_spectator.clone(),
                                ) {
                                    Ok(_game_id) => {
                                        // Запускаем announce_game_bot в отдельном потоке
                                        tokio::spawn(announce_game_bot(
                                            user_id.clone(),
                                            game_day.clone(),
                                            game_time.clone(),
                                            game_location.clone(),
                                            price_player.clone(),
                                            price_spectator.clone(),
                                        ));

                                        let context = Context {
                                            header: format!(
                                                "На {} объявлена игра. Начало игры в {}. \
                                    Место проведения игры: {}.",
                                                game_day, game_time, game_location
                                            ),
                                        };
                                        Ok(Template::render("loggedin", &context))
                                    }
                                    _ => {
                                        let context = Context {
                                            header: "В одной локации не может быть назначена игра на то же время.".to_string(),
                                        }; //прикрутить проверку времени начала и окончания игры
                                        Err(Template::render("404", &context))
                                    }
                                }
                            }
                            Err(err) => {
                                let context = Context {
                                    header: format!("Обработка ошибки парсинга даты: {}", err),
                                };
                                Err(Template::render("404", &context))
                            }
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

#[post("/attach_questions_pac_to_game/<questions_pac_id>/<game_id>")]
pub async fn attach_questions_pac_to_game(questions_pac_id: i64, game_id: i64) -> Redirect {
    println!(
        "ID игры {}. ID пакета вопросов {}",
        game_id, questions_pac_id
    );

    let connection = establish_connection();

    connection
        .execute(
            "UPDATE register_games SET package_id = ? WHERE id = ?",
            params![questions_pac_id, game_id],
        )
        .expect("Ошибка обновления package_id в таблице register_games");

    Redirect::to("/navigate_prepare_game") // После обновления перенаправляем организатора на страницу announce_game
}

#[post("/home_organiser")]
pub fn go_to_home_organiser_page(cookies: &CookieJar) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let context = Context {
                            header: "Вы вошли в систему".to_string(),
                        };
                        Template::render("loggedin", &context)
                    }
                    _ => {
                        // Пользователь не аутентифицирован, перейдите на главную страницу
                        let context = Context {
                            header: "Вы не организатор игры".to_string(),
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

#[post("/sequence_questions/<questions_pac_id>")] //изменить название функции!!!
pub async fn sequence_questions(cookies: &CookieJar<'_>, questions_pac_id: i64) -> Template {
    println!("создаём схему вопросов пакета {}", questions_pac_id);
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            match get_user_role(user_id) {
                //определение роли юзера
                Ok(role) => match role.as_str() {
                    "organiser" => {
                        let city = get_organiser_city(user_id);

                        let package_name = get_package_name(questions_pac_id);

                        let context = PackageNameContext {
                            header: city,
                            header_pac_id: questions_pac_id,
                            header_pac_name: package_name,
                        };

                        Template::render("schema_questions_pac", &context)
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

//проверяем наличие таблицы schema_questions и если она есть - удаляем
#[post("/random_topic/<questions_pac_id>")]
pub fn random_topic(cookies: &CookieJar<'_>, questions_pac_id: i64) -> Template {
    match get_user_id_from_cookies(cookies) {
        Ok(user_id) => {
            println!(
                "запуск user_{}, questions_pac_{}",
                user_id, questions_pac_id
            );

            let table_name = format!("schema_questions_{}_{}", user_id, questions_pac_id);

            let conn = establish_connection();

            if table_exists_schema_questions(table_name.clone()) {
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
                        "не удалось удалить таблицу schema_questions_{}_{} в random_topic",
                        user_id, questions_pac_id
                    ));
            }

            let context = Context {
                header: "Темы будут распределены в случайном порядке при старте игры".to_string(),
            };
            Template::render("index", &context)
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
