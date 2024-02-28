use methods::{ZK_PROVER_ELF, ZK_PROVER_ID};
use shared::types::{ZkCommit, ZkvmInput, ScriptLang};
use risc0_zkvm::{
    ExecutorEnv, default_prover,
    serde::to_vec,
};
use std::{
    time::Instant,
    sync::{Mutex, Arc},
    collections::{VecDeque, HashMap},
};
use axum::{
    routing::{Router, post, get},
    extract::{State, Path},
    Json, http::StatusCode
};
use serde::{Serialize, Deserialize};
use serde_json::ser::to_string;
use base64ct::{Base64, Encoding};


#[derive(Serialize)]
pub struct GenProofResponse {
    task_id: usize,
    active_task: usize,
}

#[derive(Serialize, Clone)]
pub enum GetStatusResponse {
    Pending(PendingProofResponse),
    Ready(String),
    Unknown,
}

#[derive(Serialize, Clone)]
pub struct PendingProofResponse {
    current_task: usize,
    time_estimate_minutes: usize,
}

#[derive(Serialize, Clone)]
pub struct ReadyProofResponse {
    proof: String,
    journal: Option<ZkCommit>,
}

#[derive(Deserialize, Clone)]
pub struct GenProofArgs {
    credentials: Vec<String>,
    lang: ScriptLang,
    script: String,
}

pub type AppState = Arc<Mutex<SharedData>>;

#[derive(Clone)]
pub struct SharedData {
    pub is_active: bool,
    pub next_id: usize,
    pub current_task: Option<usize>,
    // map taskID => task info
    pub tasks: HashMap<usize, GenProofArgs>,
    // queue of pending 
    pub pending: VecDeque<usize>,
    pub results: HashMap<usize, String>
}


pub fn proof_router() -> Router {
    let state = Arc::new(Mutex::new(SharedData {
            is_active: false,
            next_id: 0,
            current_task: Option::None,
            tasks: HashMap::new(),
            pending: VecDeque::new(),
            results: HashMap::new(),
        }));

    Router::new()
        .route("/generate", post(genproof_handler))
        .route("/status/:task_id", get(status_handler))
        .with_state(state)
}

async fn genproof_handler(
    State(app_state): State<AppState>,
    Json(payload): Json<GenProofArgs>
) -> (StatusCode, Json<GenProofResponse>) {
    // Insert the request into the pending queue
    let (was_active, task_id, active_task) = {
        // MUTEX ACQUIRED
        let mut state = app_state.lock().expect("mutex was poisoned");
        // the ID to be assigned to this task
        let task_id = state.next_id;
        state.next_id += 1;
        // are we currently generating some other proof?
        let was_active = state.is_active;
        state.tasks.insert(task_id, payload);
        state.pending.push_front(task_id);
        state.is_active = true;
        (
            was_active,
            task_id,
            if was_active { state.current_task.unwrap() } else { task_id }
        )

        // MUTEX RELEASED
    };

    // start proof generation
    if !was_active {
        tokio::spawn(async move {
            println!("Starting a new thread...");

            loop {
                // get a task from the pending queue
                let maybe_task_info: Option<(usize, GenProofArgs)> = {
                    // MUTEX ACQUIRED
                    let mut state = app_state.lock().expect("mutex was poisoned");
                    match state.pending.pop_back() {
                        Some(task_id) => {
                            match state.tasks.get(&task_id).cloned() {
                                Some(task_info) => {
                                    // update ID of currently active task
                                    state.is_active = true;
                                    state.current_task = Some(task_id);
                                    // return task ID and info
                                    Option::Some((task_id, task_info))
                                },
                                None => {
                                    state.is_active = false;
                                    Option::None
                                },
                            }
                        },
                        None => {
                            state.is_active = false;
                            Option::None
                        },
                    }

                    // MUTEX RELEASED
                };

                // No tasks remaining
                if maybe_task_info.is_none() { break; }

                // We have a task
                let (task_id, task_info) = maybe_task_info.unwrap();

                // First, we construct an executor environment
                let env = ExecutorEnv::builder()
                    .write(&ZkvmInput {
                        credentials: task_info.credentials,
                        lang: task_info.lang,
                        script: task_info.script,
                    })
                    .unwrap()
                    .build()
                    .unwrap();

                // Obtain the local prover.
                let prover = default_prover();

                let start_time_prover = Instant::now();

                // Produce a receipt by proving the specified ELF binary.
                let receipt = prover
                    .prove_elf(env, ZK_PROVER_ELF)
                    .unwrap();

                println!("Prover duration {:?}", start_time_prover.elapsed());
                println!("Receipt size {:.2} (KB)", (to_vec(&receipt).unwrap().len() / 1024));

                // Get guest result
                let code_result: ZkCommit = receipt.journal.decode().unwrap();
                println!("Result: {:?}", to_string(&code_result));

                // Verify receipt to confirm that recipients will also be able to verify it
                let start_time_verifier = Instant::now();
                receipt.verify(ZK_PROVER_ID).unwrap();
                println!("Verifier duration {:?}", start_time_verifier.elapsed());

                // save the proof as a task result
                {
                    // MUTEX ACQUIRED
                    let mut state = app_state.lock().expect("mutex was poisoned");
                    state.results.insert(
                        task_id,
                        Base64::encode_string(&bincode::serialize(&receipt).unwrap())
                    );

                    // MUTEX RELEASED
                }
            }
        });
    }

    (
        StatusCode::ACCEPTED,
        Json(GenProofResponse { task_id, active_task })
    )
}

pub async fn status_handler(State(app_state): State<AppState>, Path(task_id): Path<usize>) -> (StatusCode, Json<GetStatusResponse>) {
    let (raw_result, current_task, is_pending) = {
        // MUTEX ACQUIRED
        let state = app_state.lock().expect("mutex was poisoned");
        let raw_result: Option<String> = state.results.get(&task_id).cloned();
        let is_pending = if state.pending.contains(&task_id) {
            true
        } else {
            match state.current_task {
                Some(current_task_id) => task_id == current_task_id,
                None => false,
            }
        };

        (raw_result, state.current_task, is_pending)

        // MUTEX RELEASED
    };
    let response = match raw_result {
        Some(proof) => GetStatusResponse::Ready(proof),
        _ => {
            if is_pending {
                // We unwrap() cause it shouldn't be possible to is_pending == true, but no active task
                let current_task_id = current_task.unwrap();
                GetStatusResponse::Pending(PendingProofResponse { current_task: current_task_id, time_estimate_minutes: (task_id - current_task_id) * 2 })   
            }
            else {
                GetStatusResponse::Unknown
            }
        },
    };
    (StatusCode::ACCEPTED, Json(response))
}
