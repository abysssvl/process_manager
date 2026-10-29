use log::{error, info, warn};
use simplelog::*;
use std::fs::OpenOptions;
use std::io::{self, Write};
use sysinfo::{Pid, System};

fn setup_logger() -> Result<(), Box<dyn std::error::Error>> {
    // Відкриваємо файл у режимі дописування (append), щоб не перезаписувати історію
    let log_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("process_manager.log")?;

    WriteLogger::init(
        LevelFilter::Info,
        Config::default(),
        log_file,
    )?;
    Ok(())
}

fn print_help() {
    println!("Доступні команди:");
    println!("  list       - Показати топ-20 процесів за використанням пам'яті");
    println!("  kill <PID> - Завершити процес за його ідентифікатором (PID)");
    println!("  help       - Показати це повідомлення");
    println!("  exit       - Вийти з програми");
}

fn main() {
    // Ініціалізація логера з перевіркою помилок
    if let Err(e) = setup_logger() {
        eprintln!("Не вдалося ініціалізувати логер: {}. Робота продовжується без логування у файл.", e);
    } else {
        info!("Програму Process Manager запущено");
    }

    println!("=== Кросплатформний Менеджер Процесів ===");
    print_help();

    let mut sys = System::new_all();
    let current_pid = std::process::id();

    loop {
        print!("\n> ");
        if let Err(e) = io::stdout().flush() {
            error!("Помилка виводу в консоль: {}", e);
            eprintln!("Помилка: не вдалося оновити консоль.");
        }

        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(0) => {
                // Обробка Ctrl+D / EOF (захист від нескінченного циклу)
                info!("Введення закрито (EOF), завершення роботи");
                println!("\nЗавершення роботи...");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                error!("Помилка читання вводу користувача: {}", e);
                eprintln!("Сталася помилка при читанні команди. Спробуйте ще раз.");
                continue;
            }
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        let parts: Vec<&str> = input.split_whitespace().collect();
        let command = parts[0].to_lowercase();

        match command.as_str() {
            "list" => {
                info!("Користувач запросив список процесів");
                sys.refresh_processes();
                
                // Фільтруємо лише головні процеси (ігноруємо окремі потоки LWP)
                let mut procs: Vec<_> = sys
                    .processes()
                    .iter()
                    .filter(|(_, p)| p.thread_kind().is_none())
                    .collect();

                // Сортуємо за обсягом оперативної пам'яті
                procs.sort_by(|a, b| b.1.memory().cmp(&a.1.memory()));

                println!("{:<10} | {:<30} | {:<15}", "PID", "Назва", "Пам'ять (KB)");
                println!("{:-<60}", "-");
                
                for (pid, process) in procs.iter().take(20) {
                    println!("{:<10} | {:<30} | {:<15}", 
                        pid.as_u32(), 
                        process.name(), 
                        process.memory() / 1024
                    );
                }

                let total_count = procs.len();
                if total_count > 20 {
                    println!("... та інші (всього {} процесів)", total_count);
                }

                info!("Список процесів (топ-20) успішно виведено");
            }
            "kill" => {
                if parts.len() < 2 {
                    let msg = "Не вказано PID процесу. Використання: kill <PID>";
                    eprintln!("{}", msg);
                    warn!("Спроба завершення процесу без вказання PID");
                    continue;
                }

                let pid_str = parts[1];
                match pid_str.parse::<u32>() {
                    Ok(pid_num) => {
                        // Захист від самознищення програми
                        if pid_num == current_pid {
                            eprintln!("Помилка: не можна завершити сам менеджер процесів.");
                            warn!("Спроба завершити власний процес (PID: {})", pid_num);
                            continue;
                        }

                        let pid = Pid::from_u32(pid_num);
                        sys.refresh_processes();

                        if let Some(process) = sys.process(pid) {
                            info!("Спроба завершити процес {} (PID: {})", process.name(), pid_num);
                            
                            if process.kill() {
                                println!("Сигнал завершення для процесу {} (PID: {}) успішно відправлено.", process.name(), pid_num);
                                info!("Сигнал завершення для процесу {} (PID: {}) успішно відправлено.", process.name(), pid_num);
                            } else {
                                eprintln!("Помилка: Не вдалося завершити процес {} (PID: {}). Можливо, недостатньо прав.", process.name(), pid_num);
                                error!("Не вдалося завершити процес {} (PID: {}). Брак прав або процес захищено.", process.name(), pid_num);
                            }
                        } else {
                            eprintln!("Помилка: Процес із PID {} не знайдено.", pid_num);
                            warn!("Спроба завершити неіснуючий процес (PID: {})", pid_num);
                        }
                    }
                    Err(_) => {
                        eprintln!("Помилка: '{}' не є дійсним числовим PID.", pid_str);
                        error!("Користувач ввів невалідний PID: {}", pid_str);
                    }
                }
            }
            "help" => {
                print_help();
            }
            "exit" => {
                info!("Користувач завершив роботу програми");
                println!("Завершення роботи...");
                break;
            }
            _ => {
                eprintln!("Невідома команда: '{}'. Введіть 'help' для списку команд.", command);
                warn!("Введено невідому команду: {}", command);
            }
        }
    }
}