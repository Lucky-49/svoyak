function validation_re_rec_question() {
    let question = document.getElementById("question").value.trim(); //id поля вопрос
    let answer = document.getElementById("answer").value.trim(); //id поля ответ

    if (question !== "" &&
    answer !== "" ) { //если хоть в одном из полей пусто, то конпка
        document.getElementById("re_rec_question").disabled = false; //не активна
    } else {
        document.getElementById("re_rec_question").disabled = true; //активна
    }
}


function validation_add_question_topic() {
    let question = document.getElementById("question").value.trim(); //id поля вопрос
    let answer = document.getElementById("answer").value.trim(); //id поля ответ
    let price_question = document.getElementById("price_question").value.trim(); //id поля цена вопроса

    if (question !== "" &&
        answer !== "" &&
        price_question !== "") { //если хоть в одном из полей пусто, то конпка
        document.getElementById("add_question_topic").disabled = false; //не активна
    } else {
        document.getElementById("add_question_topic").disabled = true; //активна
    }
}