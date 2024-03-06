function validation() {
    let topic_questions = document.getElementById("topic_questions").value.trim(); //id поля пятерки вопросов
    let question = document.getElementById("question").value.trim(); //id поля вопрос
    let answer = document.getElementById("answer").value.trim(); //id поля ответ
    let price_question = document.getElementById("price_question").value.trim(); //id поля цена вопроса

    if (topic_questions !== "" &&
    question !== "" &&
    answer !== "" &&
    price_question !== "") { //если хоть в одном из полей пусто, то конпка
        document.getElementById("rec_question").disabled = false; //не активна
    } else {
        document.getElementById("rec_question").disabled = true; //активна
    }
}