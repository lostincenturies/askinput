use askinput::{read_float, read_floats, read_int, read_ints, read_str};

fn main() {
    println!("=== Проверка askinput ===\n");

    // --- read_int ---
    let a = read_int("Введи одно целое число: ");
    let b = read_int("Введи другое целое число: ");
    println!("Сумма {a} и {b} равна {}\n", a + b);

    // --- read_ints (кортеж из двух) ---
    let (x, y) = read_ints("Введи два целых числа через пробел: ");
    println!("Разность {x} и {y} равна {}\n", x - y);

    // --- read_float ---
    let f = read_float("Введи одно дробное число: ");
    println!("Удвоенное: {}\n", f * 2.0);

    // --- read_floats (кортеж из двух) ---
    let (c, d) = read_floats("Введи два дробных числа через пробел: ");
    println!("{c} + {d} = {}\n", c + d);

    // --- read_str ---
    let name = read_str("Введи своё имя: ");
    println!("Привет, {name}!\n");

    println!("=== Проверка завершена ===");
}
