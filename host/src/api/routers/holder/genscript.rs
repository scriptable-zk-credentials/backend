use axum::{
    http::StatusCode,
    Json,
};
use serde::{Serialize, Deserialize};
use serde_json::to_string;
use shared::types::ScriptLang;
use async_openai::{
    types::{CreateChatCompletionRequestArgs, ChatCompletionRequestMessage, Role},
    Client,
};


#[derive(Serialize)]
pub struct GenScriptResponse {
    script: String,
}

#[derive(Deserialize)]
pub struct GenScriptArgs {
    lang: ScriptLang,
    // Description of the credentials' schemas if possible (good)
    // If not, then the crednetials themselves (OpenAI sees them)
    cred_schemes: Vec<String>,
    requirements: String,
}

// generate a script with chatGPT
pub async fn genscript_handler(Json(payload): Json<GenScriptArgs>) -> (StatusCode, Json<GenScriptResponse>) {
    let ai_client = Client::new();
    
    let mut msgs: Vec<ChatCompletionRequestMessage> = Vec::with_capacity(4);
    // system message
    msgs.push(ChatCompletionRequestMessage {
        role: Role::System,
        content: Option::Some(format!(
            "You are a helpful code generation tool. Only reply with code in the {:?} language",
            to_string(&payload.lang).unwrap(),
        )),
        name: Option::None,
        function_call: Option::None
    });
    // example task
    msgs.push(ChatCompletionRequestMessage {
        role: Role::User,
        content: Option::Some(format!(r#"
            ```credentials = [{{"age": {{"type": "number"}}}}, {{"data": {{"type": "object", "properties": {{"country": {{"type": "string"}}}}}}}}]```.
            Above are the descriptions of some objects in a variable of type Array called "credentials".
            Assume the "credentials" variable is defined and constant. Assume the objects in it are JSON parsed.
            Write a script in {:?} that only returns true if the user credentials satisfy the following requirements:
            Age is at least 18, and country is Germany.
            Write the shortest and most performant script. Only use variables if necessary, and give them single letter names.
            "#,
            to_string(&payload.lang).unwrap(),
        )),
        name: Option::None,
        function_call: Option::None
    });
    // example expected response
    msgs.push(ChatCompletionRequestMessage {
        role: Role::Assistant,
        content: Option::Some( match payload.lang {
            ScriptLang::Rhai => r#"(credentials[0]["age"] >= 18) && (credentials[1]["data"]["country"] == "Germany")"#.to_string(),
            ScriptLang::JavaScript => r#"(credentials[0]["age"] >= 18) && (credentials[1]["data"]["country"] == "Germany")"#.to_string(),
        }),
        name: Option::None,
        function_call: Option::None
    });
    // actual task
    msgs.push(ChatCompletionRequestMessage {
        role: Role::User,
        content: Option::Some(format!(r#"
            Please do it again, but with different credentials and requirements:
            ```credentials = [{:?}]```.
            requirements:{}.
            Assume the "credentials" variable is defined and constant. Assume the objects in it are JSON parsed.
            "#,
            payload.cred_schemes.join(","),
            &payload.requirements,
        )),
        name: Option::None,
        function_call: Option::None
    });

    println!("{:?}", &msgs);

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-4")
        .messages(msgs)
        .build()
        .unwrap();
    let response = ai_client.chat().create(request).await.unwrap();

    (
        StatusCode::OK,
        Json(GenScriptResponse {
            script: response.choices[0].message.content.as_ref().unwrap().to_string(),
        })
    )
}
