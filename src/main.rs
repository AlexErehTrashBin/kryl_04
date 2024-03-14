extern crate core;

use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::io::{Error as IOError, stdin, stdout, Write};
use std::num::{ParseFloatError, ParseIntError};
use std::process::exit;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub fn get_line<'a>() -> Result<&'a str, IOError> {
    let mut result: String = String::new();
    stdin().read_line(&mut result)?;
    result = String::from(result.trim());
    let result: &'a str = result.leak::<'a>();
    Ok(result)
}

pub fn parse_to_double(str: &str) -> Result<f64, ParseFloatError> {
    Ok(f64::from_str(str)?)
}

pub fn parse_to_uint(str: &str) -> Result<u64, ParseIntError> {
    Ok(u64::from_str(str)?)
}

#[derive(Debug, Clone)]
struct IntegralCalcError<'a> {
    reason: &'a str,
}

impl<'a> IntegralCalcError<'a> {
    fn new(reason: &'a str) -> Self {
        Self {
            reason
        }
    }
}

impl<'a> Display for IntegralCalcError<'a> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Ошибка вычисления интеграла: {:}", self.reason)
    }
}

fn function(x: f64) -> f64 {
    x.atan() / (x.powi(4) + 1.0)
}

fn calculate_accumulated_sum_on_range(
    f: fn(f64) -> f64,
    lower_bound: f64,
    upper_bound: f64,
    step: f64
) -> f64 {
    let mut local_sum = 0.0;
    let mut i = lower_bound;
    while i + step < upper_bound {
        let value = f(i);
        let next_value = f(i + step);
        let median_value = (value + next_value) / 2.0;
        local_sum += median_value;
        i += step;
    }
    local_sum
}

const THREADS_COUNT: i32 = 16;

fn calculate_integral_async<'a>(
    f: fn(f64) -> f64,
    lower_bound: f64,
    upper_bound: f64,
    samples: u64,
) -> Result<f64, IntegralCalcError<'a>> {
    if lower_bound > upper_bound {
        return Err(IntegralCalcError::new("нижняя граница больше верхней"));
    }
    let range = upper_bound - lower_bound;
    let mut handles: Vec<JoinHandle<_>> = Vec::new();
    let mut current_sample = 0;
    let threads_count = THREADS_COUNT;
    let accumulated_sum = Arc::new(Mutex::new(0.0));
    while current_sample < threads_count {
        let current_lower_bound = lower_bound + current_sample as f64 * range / threads_count as f64;
        let current_upper_bound = lower_bound + (current_sample + 1) as f64 * range / threads_count as f64;
        let acc_sum_atomic_ref = accumulated_sum.clone();
        let handle = std::thread::spawn(move || {
            let local_sum = calculate_accumulated_sum_on_range(
                f,
                current_lower_bound,
                current_upper_bound,
                range / samples as f64
            );
            *acc_sum_atomic_ref.clone().lock().unwrap() += local_sum;
        });
        handles.push(handle);
        current_sample += 1;
    }


    for handle in handles {
        handle.join().unwrap();
    }

    let result = *accumulated_sum.clone().lock().unwrap() / samples as f64;
    Ok(result)
}

const MAX_SAMPLES_COUNT: u64 = 1000000000u64;
const ASYNC_THRESHOLD_SAMPLES_COUNT: u64 = 10000u64;

fn calculate_integral<'a>(
    f: fn(f64) -> f64,
    lower_bound: f64,
    upper_bound: f64,
    samples: u64,
) -> Result<f64, IntegralCalcError<'a>> {
    if lower_bound > upper_bound {
        return Err(IntegralCalcError::new("нижняя граница больше верхней"));
    }
    if samples > MAX_SAMPLES_COUNT {
        return Err(IntegralCalcError::new("превышено максимальное число отсчётов"));
    }
    if samples > ASYNC_THRESHOLD_SAMPLES_COUNT {
        return calculate_integral_async(f, lower_bound, upper_bound, samples);
    }
    let step = (upper_bound - lower_bound) / samples as f64;
    let accumulated_sum = calculate_accumulated_sum_on_range(
        f, 
        lower_bound, 
        upper_bound, 
        step
    );
    let result = accumulated_sum / samples as f64;
    Ok(result)
}


const EXIT_INCORRECT_LOWER_BOUND: i32 = 1;
const EXIT_INCORRECT_UPPER_BOUND: i32 = 2;
const EXIT_INCORRECT_SAMPLES_COUNT: i32 = 3;
const EXIT_UNABLE_TO_CALCULATE: i32 = 4;

fn main() {
    print!("Введите нижнюю границу: ");
    stdout().flush().unwrap();
    let lower_bound = parse_to_double(get_line().unwrap())
        .inspect_err(|_| {
            eprintln!("Ошибка преобразования ввода в вещественное число");
            exit(EXIT_INCORRECT_LOWER_BOUND);
        })
        .unwrap();
    print!("Введите верхнюю границу: ");
    stdout().flush().unwrap();
    let upper_bound = parse_to_double(get_line().unwrap())
        .inspect_err(|_| {
            eprintln!("Ошибка преобразования ввода в вещественное число");
            exit(EXIT_INCORRECT_UPPER_BOUND);
        })
        .unwrap();
    print!("Введите количество отсчётов: ");
    stdout().flush().unwrap();
    let samples = parse_to_uint(get_line().unwrap())
        .inspect_err(|_| {
            eprintln!("Ошибка преобразования ввода в целое число");
            exit(EXIT_INCORRECT_SAMPLES_COUNT);
        })
        .unwrap();
    let result = calculate_integral(function, lower_bound, upper_bound, samples)
        .inspect_err(|e| {
            eprintln!("{}", e);
            exit(EXIT_UNABLE_TO_CALCULATE);
        })
        .unwrap();
    println!("Вычисленное значение интеграла: {:}", result);
}
