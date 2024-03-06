function validation() {
    let package_name = document.getElementById("package_name").value.trim();
    let topic_questions = document.getElementById("topic_questions").value.trim(); //id поля пятерки вопросов
    let question = document.getElementById("question").value.trim(); //id поля вопрос
    let answer = document.getElementById("answer").value.trim(); //id поля ответ
    let price_question = document.getElementById("price_question").value.trim(); //id поля цена вопроса

    if (package_name !== "" &&
        topic_questions !== "" &&
        question !== "" &&
        answer !== "" &&
        price_question !== "") { //если хоть в одном из полей пусто, то конпка
        document.getElementById("rec_question_from_player_context").disabled = false; //не активна
    } else {
        document.getElementById("rec_question_from_player_context").disabled = true; //активна
    }
}