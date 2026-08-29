# Chapter 21: Tagged Union Enums, Pattern Matching & Error Handling

> **Part VI: The Agam Language Programming Guide**  
> **Target Audience**: Software Engineers learning Agam (Intermediate-to-Advanced)

---

## 21.1 Tagged Union Enums

Enums in Agam can carry payload data inside variant constructors:

```agam
enum Command {
    Quit,
    Move { x: Int, y: Int },
    Write(String),
    ChangeColor(Int, Int, Int),
}
```

---

## 21.2 Pattern Matching (`match`)

Pattern matching is exhaustive; every possible enum variant must be handled:

```agam
fn process_command(cmd: Command) {
    match cmd {
        Command.Quit => println("Quitting program..."),
        Command.Move { x, y } => {
            println("Moving to X: " + x.to_string() + ", Y: " + y.to_string());
        }
        Command.Write(text) => println("Writing text: " + text),
        Command.ChangeColor(r, g, b) => println("Color changed"),
    }
}
```

---

## 21.3 Robust Error Handling with `Option` and `Result`

Agam avoids `null` pointer exceptions by using explicit `Option[T]` and `Result[T, E]` types:

```agam
enum Option[T] {
    Some(T),
    None,
}

enum Result[T, E] {
    Ok(T),
    Err(E),
}

fn divide(numerator: Float, denominator: Float) -> Result[Float, String] {
    if denominator == 0.0 {
        return Result.Err("Division by zero error");
    }
    return Result.Ok(numerator / denominator);
}

fn main() {
    match divide(10.0, 2.0) {
        Result.Ok(val) => println("Division result: " + val.to_string()),
        Result.Err(err) => println("Error occurred: " + err),
    }
}
```
