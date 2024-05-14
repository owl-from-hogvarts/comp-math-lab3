use core::panic;
use std::ops::{Range, RangeBounds};

use inquire::{
    list_option::ListOption,
    validator::{ErrorMessage, Validation},
    CustomType, InquireError, Select,
};

type TNumber = f64;
const MAX_ITERATIONS: usize = 100_000;

#[derive(Clone, Copy)]
enum RectangleMode {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy)]
enum ComputationMethod {
    Rectangle(RectangleMode),
    Trapezoid,
    Sympthonm,
}

#[derive(Clone)]
struct Config {
    range: Range<TNumber>,
    number_of_splits: usize,
}

type SingleVariableFunction = fn(f64) -> f64;

const FUNCTIONS: [SingleVariableFunction; 4] = [
    |x| 2. * x.powi(3) - 2. * x.powi(2) + 7. * x - 14.,
    |x| -3. * x.powi(3) - 5. * x.powi(2) + 4. * x - 2.,
    |x| x.sin() + 1.125,
    |x| x.sqrt().sin() + 2.,
];

fn main() {
    match start() {
        Ok(_) => (),
        Err(error) => eprint!("Error: {error}"),
    }
}

fn start() -> Result<(), InquireError> {
    let function_index = Select::new(
        "Select function",
        vec![
            "2x^3 - 2x^2 + 7x - 14",
            "-3x^3 - 5x^2 + 4x - 2",
            "sin(x) + 1.125",
            "sin(sqrt(x)) + 2",
        ],
    )
    .raw_prompt()?
    .index;

    let method: ComputationMethod = {
        let index = Select::new(
            "Select method",
            vec!["Rectangle (Central, right, left)", "Trapezoid", "Sympthon"],
        )
        .raw_prompt()?
        .index;

        match index {
            0 => {
                let mode = match Select::new(
                    "Select rectangle method mode",
                    vec!["Right", "Central", "Left"],
                )
                .raw_prompt()
                {
                    Ok(ListOption { index, .. }) => match index {
                        0 => RectangleMode::Right,
                        1 => RectangleMode::Center,
                        2 => RectangleMode::Left,
                        _ => unreachable!("Unknown mode with index {index}!"),
                    },
                    Err(_) => todo!(),
                };

                ComputationMethod::Rectangle(mode)
            }
            1 => ComputationMethod::Trapezoid,
            2 => ComputationMethod::Sympthonm,
            _ => panic!("Unsupported method with index {index} method"),
        }
    };

    let start = CustomType::<f64>::new("Compute integral from")
        .with_default(0.)
        .prompt()?;

    let end = CustomType::<f64>::new("Conmpute intgral to")
        .with_default(1.)
        .prompt()?;

    let range = start..end;

    let epsilon = CustomType::<f64>::new("Epsilon (allowed divergence)")
        .with_default(0.001)
        .with_validator(|&value: &f64| {
            if value > 0. {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid(ErrorMessage::Custom(format!(
                    "Should be strictly above zero! Got {value}"
                ))))
            }
        })
        .prompt()?;

    let mut number_of_splits = CustomType::<usize>::new("Initial number of splits")
        .with_default(5)
        .with_validator(move |&value: &usize| {
            if matches!(method, ComputationMethod::Sympthonm) && value % 2 == 1 {
                return Ok(Validation::Invalid(ErrorMessage::Custom(format!(
                    "Number {value} is odd number! Should be even!"
                ))));
            }

            Ok(Validation::Valid)
        })
        .prompt()?;

    let function = FUNCTIONS[function_index];

    for _ in 0..MAX_ITERATIONS {
        let integral = compute_integral(&range, number_of_splits, method, function);
        let number_of_splits_doubled = number_of_splits * 2;
        let double_splitted = compute_integral(&range, number_of_splits_doubled, method, function);

        // deviation/error
        let divergence = runge_rule(double_splitted, integral, method);

        if divergence < epsilon {
            println!("Integral: {double_splitted}");
            println!("Numebr of splits used: {number_of_splits_doubled}");
            return Ok(());
        }

        number_of_splits = number_of_splits_doubled * 2;
    }

    #[rustfmt::skip]
    panic!(
r"Number of interations to reach prompted precision exceeds maximum interations!
Max iterations: {MAX_ITERATIONS}; number of splits reached: {number_of_splits}
Try to specify higher initial number of splits or lower precision"
    );
}

fn compute_integral(
    range: &Range<f64>,
    number_of_splits: usize,
    method: ComputationMethod,
    function: SingleVariableFunction,
) -> f64 {
    let config = Config {
        range: range.clone(),
        number_of_splits,
    };
    match method {
        ComputationMethod::Rectangle(mode) => solve_rectanlge(&config, mode, function),
        ComputationMethod::Trapezoid => solve_trapezoid(&config, function),
        ComputationMethod::Sympthonm => solve_simpthon(&config, function),
    }
}

fn solve_rectanlge(config: &Config, mode: RectangleMode, function: SingleVariableFunction) -> f64 {
    let step_length = compute_step_length(&config);

    let half_step = step_length / 2.;

    let mut integral_sum: f64 = 0.;

    for left_border_multiplier in 0..config.number_of_splits {
        let left_bound = config.range.start + step_length * left_border_multiplier as f64;

        let height: f64 = match mode {
            RectangleMode::Left => function(left_bound),
            RectangleMode::Center => {
                let center = left_bound + half_step;
                function(center)
            }
            RectangleMode::Right => {
                let right_bound = left_bound + step_length;
                function(right_bound)
            }
        };

        let area = height * step_length;
        integral_sum += area;
    }

    integral_sum
}

fn solve_trapezoid(config: &Config, function: SingleVariableFunction) -> f64 {
    let step_length = compute_step_length(&config);

    let first_point = function(config.range.start);
    let end_point = function(config.range.end);
    let mut sum_of_intermediate_points: f64 = 0.;

    for point_index in 1..config.number_of_splits {
        let point_x = config.range.start + step_length * point_index as f64;
        let height = function(point_x);
        sum_of_intermediate_points += height;
    }

    step_length / 2. * (first_point + end_point + 2. * sum_of_intermediate_points)
}

fn solve_simpthon(config: &Config, function: SingleVariableFunction) -> f64 {
    if config.number_of_splits % 2 != 0 {
        panic!("number of splits should be even");
    }

    let step_length = compute_step_length(&config);
    let first_point = function(config.range.start);
    let end_point = function(config.range.end);

    let mut odd_sum: f64 = 0.;
    let mut even_sum: f64 = 0.;

    for index in 1..config.number_of_splits {
        let point_x = config.range.start + step_length * index as f64;
        let height = function(point_x);

        if index % 2 == 0 {
            even_sum += height;
        } else {
            odd_sum += height;
        }
    }

    step_length / 3. * (first_point + 4. * odd_sum + 2. * even_sum + end_point)
}

fn compute_step_length(
    Config {
        range,
        number_of_splits,
        ..
    }: &Config,
) -> f64 {
    let range_length = range.len();
    range_length / *number_of_splits as f64
}

trait FloatRangeLength: RangeBounds<f64> {
    fn len(&self) -> f64;
}

impl FloatRangeLength for Range<f64> {
    fn len(&self) -> f64 {
        self.end - self.start
    }
}

fn runge_rule(half: f64, full: f64, method: ComputationMethod) -> f64 {
    let k = match method {
        ComputationMethod::Rectangle(_) | ComputationMethod::Trapezoid => 2,
        ComputationMethod::Sympthonm => 4,
    };

    (half - full) / (2_f64.powi(k) - 1.)
}
