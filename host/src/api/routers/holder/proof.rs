use methods::{ZK_PROVER_ELF, ZK_PROVER_ID};
use shared::types::{ZkCommit, ZkvmInput, ScriptLang};
use risc0_zkvm::{
    Executor, ExecutorEnv,
    serde::{to_vec, from_slice},
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
    // TODO: Add status of Unknown tasks (not in Pending queue, and not in finished results)
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
    pub current_task: usize,
    // queue of pending 
    pub pending: VecDeque<GenProofTask>,
    pub results: HashMap<usize, String>
}

#[derive(Clone)]
pub struct GenProofTask {
    id: usize,
    params: GenProofArgs,
}


pub fn proof_router() -> Router {
    let state = Arc::new(Mutex::new(SharedData {
            is_active: false,
            next_id: 0,
            current_task: 0, // start value does not matter, because of how we re-assign the variable each time
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
        let mut state = app_state.lock().expect("mutex was poisoned");
        // the ID to be assigned to this task
        let next_id = state.next_id;
        // are we currently generating some other proof?
        let was_active = state.is_active;
        // if yes, what's the task ID associated with it
        let active_task_id = state.current_task;
        state.pending.push_front(GenProofTask { id: next_id, params: payload });
        state.next_id += 1;
        state.is_active = true;
        (
            was_active,
            next_id,
            if was_active { active_task_id } else { next_id }
        )
    };

    // start proof generation
    if !was_active {
        tokio::spawn(async move {
            loop {
                // get a task from the pending queue
                let maybe_task = {
                    let mut state = app_state.lock().expect("mutex was poisoned");
                    let maybe_task = state.pending.pop_back();
                    let maybe_task_copy = maybe_task.clone();
                    // update ID of currently active task
                    match maybe_task {
                        Some(task) => {
                            state.is_active = true;
                            state.current_task = task.id;
                        },
                        None => { state.is_active = false; }
                    }

                    maybe_task_copy
                };

                // No tasks remaining
                if maybe_task.is_none() { break; }

                // We have a task
                let task = maybe_task.unwrap();

                // First, we construct an executor environment
                let env = ExecutorEnv::builder()
                    .add_input(&to_vec(&ZkvmInput {
                        credentials: task.params.credentials,
                        lang: task.params.lang,
                        script: task.params.script,
                    }).unwrap())
                    .build()
                    .unwrap();

                // Next, we make an executor, loading the (renamed) ELF binary.
                let mut exec = Executor::from_elf(env, ZK_PROVER_ELF).unwrap();

                // Run the executor to produce a session.
                let session = exec.run().unwrap();

                let start_time_prover = Instant::now();

                // Prove the session to produce a receipt.
                let receipt = session.prove().unwrap();

                println!("Prover duration {:?}", start_time_prover.elapsed());
                println!("Receipt size {:.2} (KB)", (to_vec(&receipt).unwrap().len() / 1024));

                // Get guest result
                let code_result: ZkCommit = from_slice(&receipt.journal).unwrap();
                println!("Result: {:?}", to_string(&code_result));

                // Verify receipt to confirm that recipients will also be able to verify it
                let start_time_verifier = Instant::now();
                receipt.verify(ZK_PROVER_ID).unwrap();
                println!("Verifier duration {:?}", start_time_verifier.elapsed());

                // save the proof as a task result
                {
                    let mut state = app_state.lock().expect("mutex was poisoned");
                    state.results.insert(
                        task.id,
                        Base64::encode_string(&bincode::serialize(&receipt).unwrap())
                    );
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
    let (raw_result, current_task) = {
        let state = app_state.lock().expect("mutex was poisoned");
        let raw_result: Option<String> = state.results.get(&task_id).cloned();
        (raw_result, state.current_task)
    };
    let response = match raw_result {
        Some(proof) => GetStatusResponse::Ready(proof),
        _ => GetStatusResponse::Pending(PendingProofResponse { current_task, time_estimate_minutes: (task_id - current_task) * 2 })
    };
    (StatusCode::ACCEPTED, Json(response))
}
