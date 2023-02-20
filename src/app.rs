use std::{
    borrow::Cow,
    fmt::{Debug, Display},
    ops::{Deref, RangeInclusive},
    path::{Path, PathBuf},
    rc::Rc,
    str::FromStr,
};

use chrono::{Datelike, Local};
use futures::StreamExt;
use gloo_utils::format::JsValueSerdeExt;
use num_traits::cast::FromPrimitive;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;
use shared::{Credentials, Message, Month, YearMonth};
use tauri_sys::event;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "tauri"], catch)]
    async fn invoke(cmd: &str, args: JsValue) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "dialog"])]
    async fn open(options: JsValue) -> JsValue;

    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartArgs {
    creds: Credentials,
    range: RangeInclusive<YearMonth>,
    save_location: PathBuf,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Progress {
    processed: usize,
    total: usize,
}

impl Progress {
    fn new(range: &RangeInclusive<YearMonth>) -> Self {
        let total = YearMonth::iter_range(range).count();
        Self {
            processed: 0,
            total,
        }
    }

    fn percent(&self) -> f64 {
        self.processed as f64 / self.total as f64 * 100.0
    }

    fn processed(&self) -> usize {
        self.processed
    }

    fn total(&self) -> usize {
        self.total
    }
}

impl Display for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.0}%", self.percent())
    }
}

enum ProgressAction {
    Increment,
    SetTotal(usize),
    Reset,
}

impl Reducible for Progress {
    type Action = ProgressAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut processed = self.processed;
        let mut total = self.total;
        match action {
            ProgressAction::Increment => processed += 1,
            ProgressAction::SetTotal(new) => {
                if new > 0 {
                    total = new;
                }
            }
            ProgressAction::Reset => processed = 0,
        }
        Self { processed, total }.into()
    }
}

/// Contents of the output view
#[derive(Debug, Default, PartialEq, Eq)]
struct Output(Vec<Message>);

impl Output {
    fn iter(&self) -> impl Iterator<Item = &Message> + '_ {
        self.0.iter()
    }
}

impl Display for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

enum OutputAction {
    Message(Message),
    Clear,
}

impl Reducible for Output {
    type Action = OutputAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        let mut output = self.0.clone();
        match action {
            OutputAction::Message(message) => {
                output.push(message);
            }
            OutputAction::Clear => {
                output.clear();
            }
        }
        Self(output).into()
    }
}

#[function_component(App)]
pub fn app() -> HtmlResult {
    let login_email_ref = use_node_ref();
    let login_password_ref = use_node_ref();
    let range_from_ref = use_node_ref();
    let range_to_ref = use_node_ref();

    let range_from = use_state_eq(|| YearMonth::from_str("2018-04").unwrap());
    let range_to = use_state_eq(|| {
        let today = Local::now().date_naive();
        YearMonth {
            year: today.year(),
            month: Month::from_u32(today.month()).unwrap(),
        }
    });

    let running = use_state_eq(|| false);
    let progress = use_reducer_eq(|| Progress::new(&(*range_from..=*range_to)));
    let output = use_reducer_eq(Output::default);
    {
        let output = output.clone();
        let progress = progress.clone();
        use_effect_with_deps(
            move |_| {
                let output = output.clone();
                let progress = progress.clone();
                spawn_local(async move {
                    let mut stream = event::listen::<Message>("output").await.unwrap();
                    while let Some(message) = stream.next().await {
                        if let Message::Complete(_) = &message.payload {
                            progress.dispatch(ProgressAction::Increment);
                        }
                        output.dispatch(OutputAction::Message(message.payload));
                    }
                });
            },
            (),
        );
    }

    let save_location = use_state_eq(|| None);
    {
        let save_location = save_location.clone();
        use_effect_with_deps(
            |_| {
                spawn_local(async move {
                    let ret = invoke("default_save_location", to_value(&()).unwrap()).await;
                    log(&format!("{:?}", &ret));
                    if let Ok(val) = ret {
                        if let Some(str) = val.as_string() {
                            save_location.set(Some(PathBuf::from(str)));
                        }
                    }
                });
            },
            (),
        );
    }

    let change_save_location = {
        let save_location = save_location.clone();
        Callback::from(move |_| {
            let save_location = save_location.clone();
            #[derive(Serialize)]
            #[serde(rename_all = "camelCase")]
            struct Opts<'a> {
                default_path: Option<&'a Path>,
                directory: bool,
            }
            let opts = <JsValue as JsValueSerdeExt>::from_serde(&Opts {
                default_path: save_location.as_deref(),
                directory: true,
            })
            .unwrap();
            spawn_local(async move {
                save_location.set(open(opts).await.as_string().as_ref().map(PathBuf::from));
            });
        })
    };
    let update_range = |state: UseStateHandle<YearMonth>, node: NodeRef| {
        let state = state.clone();
        let node = node.clone();
        Callback::from(move |_event: Event| {
            let state = state.clone();
            if let Some(element) = node.cast::<web_sys::HtmlInputElement>() {
                let str = element.value();
                if let Ok(month) = YearMonth::from_str(&str) {
                    state.set(month);
                }
            }
        })
    };
    {
        let range_from = range_from.clone();
        let range_to = range_to.clone();
        let progress = progress.clone();
        use_effect_with_deps(
            move |(range_from, range_to)| {
                let total = YearMonth::iter_range(&(**range_from..=**range_to)).count();
                progress.dispatch(ProgressAction::SetTotal(total));
            },
            (range_from, range_to),
        );
    }

    let start = {
        let save_location = save_location.clone();
        let running = running.clone();
        let progress = progress.clone();
        let output = output.clone();
        let login_email_ref = login_email_ref.clone();
        let login_password_ref = login_password_ref.clone();
        let range_from_ref = range_from_ref.clone();
        let range_to_ref = range_to_ref.clone();
        Callback::from(move |_| {
            if *running {
                return;
            }
            let running = running.clone();
            let range_from_ref = range_from_ref.clone();
            let range_to_ref = range_to_ref.clone();
            let save_location = save_location.deref().clone();
            let output = output.clone();
            let login_email_ref = login_email_ref.clone();
            let login_password_ref = login_password_ref.clone();
            macro_rules! or_report {
                ( $name:expr, $x:expr $(,)? ) => {
                    match $x {
                        Some(v) => v,
                        None => {
                            output.dispatch(OutputAction::Message(Message::error(format!(
                                "不正な{}です。",
                                $name,
                            ))));
                            return;
                        }
                    }
                };
            }
            let creds = Credentials {
                email: or_report!(
                    "メールアドレス",
                    login_email_ref.cast::<web_sys::HtmlInputElement>()
                )
                .value(),
                password: or_report!(
                    "パスワード",
                    login_password_ref.cast::<web_sys::HtmlInputElement>()
                )
                .value(),
            };
            let from = YearMonth::from_str(
                &or_report!("年月", range_from_ref.cast::<web_sys::HtmlInputElement>()).value(),
            )
            .unwrap();
            let to = YearMonth::from_str(
                &or_report!("年月", range_to_ref.cast::<web_sys::HtmlInputElement>()).value(),
            )
            .unwrap();
            let range = from..=to;
            running.set(true);
            progress.dispatch(ProgressAction::Reset);
            output.dispatch(OutputAction::Clear);
            spawn_local(async move {
                let message = invoke(
                    "start",
                    to_value(&StartArgs {
                        creds,
                        range,
                        save_location: save_location.unwrap(),
                    })
                    .unwrap(),
                )
                .await
                .map(|val| val.as_string())
                .map_err(|err| err.as_string());
                let message = match message {
                    Ok(Some(message)) => Some(Message::message(message)),
                    Err(Some(err)) => Some(Message::error(err)),
                    _ => None,
                };
                if let Some(message) = message {
                    output.dispatch(OutputAction::Message(message));
                }
                running.set(false);
            });
        })
    };

    let percent = format!("width: {:.0}%", progress.percent());
    let processed = progress.processed();
    let total = progress.total();
    let download_button_classes = if *running {
        classes!("bg-gray-300", "disabled", "cursor-wait")
    } else {
        classes!(
            "bg-indigo-600",
            "hover:bg-indigo-700",
            "focus:outline-none",
            "focus:ring-2",
            "focus:ring-indigo-500",
            "focus:ring-offset-2",
        )
    };
    Ok(html! {
        <main class="flex flex-col h-screen bg-white sm:rounded-lg">
          <div class="w-full border-b border-gray-200">
            <dl>
              <div class="bg-gray-50 px-4 py-5 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6 items-center">
                <dt class="text-sm font-medium text-gray-500">{"ログインメールアドレス"}</dt>
                <dd class="mt-1 text-sm text-gray-900 sm:col-span-2 sm:mt-0">
                  <input id="login-email" placeholder="you@example.com" class="w-full h-10 px-2 border-2 border-indigo-600/50 rounded-md" ref={login_email_ref} type="email" />
                </dd>
              </div>
              <div class="bg-white px-4 py-5 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6 items-center">
                <dt class="text-sm font-medium text-gray-500">{"ログインパスワード"}</dt>
                <dd class="mt-1 text-sm text-gray-900 sm:col-span-2 sm:mt-0">
                  <input id="login-password" placeholder="Type your password" class="w-full h-10 px-2 border-2 border-indigo-600/50 rounded-md" ref={login_password_ref} type="password" />
                </dd>
              </div>
              <div class="bg-gray-50 px-4 py-5 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6 items-center">
                <dt class="text-sm font-medium text-gray-500">{"期間"}</dt>
                <dd class="mt-1 text-sm text-gray-900 sm:col-span-2 sm:mt-0">
                  <div class="flex justify-between items-center">
                    <input id="range-from" class="h-10 text-center border-2 border-indigo-600/50 rounded-md" ref={range_from_ref.clone()} type="month" value={format!("{}", &*range_from)} onchange={update_range(range_from, range_from_ref)} />
                    <span class="text-bottom">{"〜"}</span>
                    <input id="range-to" class="h-10 text-center border-2 border-indigo-600/50 rounded-md" ref={range_to_ref.clone()} type="month" value={format!("{}", &*range_to)} onchange={update_range(range_to, range_to_ref)} />
                  </div>
                </dd>
              </div>
              <div class="bg-white px-4 py-5 sm:grid sm:grid-cols-3 sm:gap-4 sm:px-6 items-center">
                <dt class="text-sm font-medium text-gray-500">{"保存先"}</dt>
                <dd class="mt-1 text-sm text-gray-900 sm:col-span-2 sm:mt-0">
                  <div class="flex items-center justify-between pl-3 pr-4 text-sm">
                    <div class="flex w-0 flex-1 items-center">
                      <span class="w-0 flex-1 truncate">
                        { if let Some(location) = &*save_location {
                            format!("{}", location.display())
                        } else {
                            "未設定".into()
                        }}
                      </span>
                    </div>
                    <div class="ml-4 flex-shrink-0">
                      <button class="bg-white hover:bg-gray-100 text-indigo-600 py-2 px-4 rounded shadow" onclick={change_save_location}>{"変更"}</button>
                    </div>
                  </div>
                </dd>
              </div>
            </dl>
          </div>
          <div class="w-full flex justify-center my-8">
            <button class={classes!("group", "relative", "flex", "w-80", "items-center", "justify-center", "rounded-md", "border", "border-transparent", "py-2", "px-4", "text-sm", "font-medium", "text-white", download_button_classes)} type="button" onclick={start} disabled={*running} >
              <svg class="fill-current w-4 h-4 mr-2" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20"><path d="M13 8V2H7v6H2l8 8 8-8h-5zM0 18h20v2H0v-2z"/></svg>
              <span>{ if *running { "ダウンロード中..." } else { "ダウンロード" } }</span>
            </button>
          </div>
          <div class="w-full flex justify-center items-center my-6">
            <div class="w-5/6 bg-gray-200 rounded-full h-1.5">
              <div class="flex-grow bg-indigo-600 h-1.5 rounded-full" style={percent}></div>
            </div>
            <span class="w-14 flex-none pl-5">{processed} {"/"} {total}</span>
          </div>
          <div class="h-full grow m-5 bg-gray-800 overflow-y-scroll rounded-lg">
            <div class="h-full p-3 text-gray-100">
            {
                output.iter().enumerate().map(|(i, message)| {
                    let text = match message {
                        Message::Message(message) => Cow::from(message),
                        Message::Error(err) => err.into(),
                        Message::Complete(month) => format!("{month}").into(),
                    };
                    html! { <div key={i} class={classes!(message.is_err().then_some("text-red-400"))}>{ text }</div> }
                }).collect::<Html>()
            }</div>
          </div>
        </main>
    })
}
