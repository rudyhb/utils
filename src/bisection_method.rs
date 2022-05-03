pub struct Options {
    pub epsilon: f64,
    pub expand_to: ExpandDirection,
}

pub enum ExpandDirection {
    None,
    Left,
    Right,
}

pub fn get_zero<TFun: Fn(f64) -> f64>(
    func: TFun,
    left: f64,
    right: f64,
    options: &Options,
) -> Option<f64> {
    let mut left = left;
    let mut right = right;
    let mut value_left = func(left);
    let mut _value_right = func(right);

    while (value_left * _value_right).is_sign_positive() {
        match options.expand_to {
            ExpandDirection::None => return None,
            ExpandDirection::Left => {
                left -= 10.0 * (right - left).abs();
                if !left.is_normal() {
                    return None;
                }
                value_left = func(left);
            }
            ExpandDirection::Right => {
                right += 10.0 * (right - left).abs();
                if !right.is_normal() {
                    return None;
                }
                _value_right = func(right);
            }
        }
    }

    loop {
        let guess = (left + right) / 2.0;
        let value_guess = func(guess);

        if value_guess.abs() < options.epsilon {
            return Some(guess);
        } else if (value_left * value_guess).is_sign_positive() {
            left = guess;
            value_left = value_guess;
        } else {
            right = guess;
            _value_right = value_guess;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_get_zero() {
        let func = |x: f64| -> f64 { x.powi(3) - 2.0 * x.powi(2) + 13.0 };

        let options = Options {
            epsilon: 0.001,
            expand_to: ExpandDirection::None,
        };

        let zero = get_zero(func, -5.0, 5.0, &options).unwrap();

        let solution = -1.84f64;
        let round = |n: f64| (n * 1000.0).floor() / 1000.0;
        assert_eq!(round(zero), round(solution));
    }

    #[test]
    fn should_get_zero_unbounded_right() {
        let func = |x: f64| -> f64 { x.powi(2) - 20.0 };

        let options = Options {
            epsilon: 0.001,
            expand_to: ExpandDirection::Right,
        };

        let zero = get_zero(func, -4.4, 1.0, &options).unwrap();

        let solution = 5f64.sqrt() * 2.0;
        let round = |n: f64| (n * 1000.0).round() / 1000.0;
        assert_eq!(round(zero), round(solution));
    }
}
