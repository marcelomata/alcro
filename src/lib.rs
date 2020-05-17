use std::sync::mpsc;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

mod chrome;
use chrome::Chrome;

const DEFAULT_CHROME_ARGS: [&str; 25] = [
    "--disable-background-networking",
    "--disable-background-timer-throttling",
    "--disable-backgrounding-occluded-windows",
    "--disable-breakpad",
    "--disable-client-side-phishing-detection",
    "--disable-default-apps",
    "--disable-dev-shm-usage",
    "--disable-infobars",
    "--disable-extensions",
    "--disable-features=site-per-process",
    "--disable-hang-monitor",
    "--disable-ipc-flooding-protection",
    "--disable-popup-blocking",
    "--disable-prompt-on-repost",
    "--disable-renderer-backgrounding",
    "--disable-sync",
    "--disable-translate",
    "--disable-windows10-custom-titlebar",
    "--metrics-recording-only",
    "--no-first-run",
    "--no-default-browser-check",
    "--safebrowsing-disable-auto-update",
    "--enable-automation",
    "--password-store=basic",
    "--use-mock-keychain",
];

pub struct UI {
    chrome: Chrome,
    tmpdir: Option<tempdir::TempDir>,
    kill_send: mpsc::Sender<()>,
    done: Arc<AtomicBool>, //TODO:Add channel
}

impl UI {
    pub fn new(url: &str, dir: &str, width: u32, height: u32, customArgs: &[&str]) -> UI {
        let url = if url.is_empty() {
            "data:text/html,<html></html>"
        } else {
            url
        };

        let tmpdir = if dir.is_empty() {
            let t = tempdir::TempDir::new("alcro").expect("Cannot create temp directory");
            Some(t)
        } else {
            None
        };

        let dir = if dir.is_empty() {
            tmpdir.as_ref().unwrap().path().to_str().unwrap()
        } else {
            dir
        };

        let mut args: Vec<String> = Vec::from(&DEFAULT_CHROME_ARGS[..])
            .into_iter()
            .map(|s| s.to_string())
            .collect();
        args.push(format!("--app={}", url));
        args.push(format!("--user-data-dir={}", dir));
        args.push(format!("--window-size={},{}", width, height));
        for arg in customArgs {
            args.push(arg.to_string())
        }
        args.push("--remote-debugging-port=0".to_string());

        let mut chrome = Chrome::new_with_args("/usr/bin/google-chrome", args); //TODO:Make chrome exe fn
        let done = Arc::new(AtomicBool::new(false));

        let (kill_send, kill_recv) = mpsc::channel();
        let done_cloned = Arc::clone(&done);
        let mut chrome_cmd = chrome.cmd.take().unwrap();
        std::thread::spawn(move || {
            loop {
                if chrome_cmd.try_wait().expect("Error in waiting").is_some() {
                    done_cloned.store(true, Ordering::SeqCst);
                    break;
                } else if kill_recv.try_recv().is_ok() {
                    //TODO:Close websocket
                    chrome_cmd.kill();
                    done_cloned.store(true, Ordering::SeqCst);
                    break;
                }
            }
        });
        UI {
            chrome,
            done,
            tmpdir,
            kill_send,
        }
    }

    pub fn done(&self) -> bool {
        return self.done.load(Ordering::SeqCst);
    }

    pub fn close(&self) {
        if !self.done() {
            self.kill_send.send(()).expect("Receiver end closed");
        }
    }
}
