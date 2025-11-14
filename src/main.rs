use std::process::ExitCode;

use wtw::error::AppError;

fn main() -> ExitCode {
    match wtw::run() {
        Ok(code) => code,
        Err(error) => match error.downcast::<AppError>() {
            Ok(app) => {
                eprintln!("{app}");
                ExitCode::from(app.exit_code())
            }
            Err(error) => {
                let mut exit_code = 10u8;
                let mut message = error.to_string();

                for cause in error.chain() {
                    if let Some(app) = cause.downcast_ref::<AppError>() {
                        exit_code = app.exit_code();
                        message = app.to_string();
                        break;
                    } else if cause.is::<wtw::git::runner::GitError>() {
                        exit_code = 3;
                        message = cause.to_string();
                        break;
                    } else if cause.is::<serde_yaml::Error>() {
                        exit_code = 2;
                        message = cause.to_string();
                        break;
                    }
                }

                eprintln!("{message}");
                ExitCode::from(exit_code)
            }
        },
    }
}
