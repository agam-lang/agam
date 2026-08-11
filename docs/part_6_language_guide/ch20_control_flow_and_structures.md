# Chapter 20: Control Flow, Structs & Collections

> **Part VI: The Agam Language Programming Guide**  
> **Target Audience**: Software Engineers learning Agam (Intermediate Level)

---

## 20.1 Control Flow: Conditionals & Loops

### 1. `if` Expression
In Agam, `if` is an expression that returns a value:

```agam
fn main() {
    let score = 85;
    
    // Conditionals return values directly
    let status = if score >= 50 {
        "Passed"
    } else {
        "Failed"
    };

    println("Status: " + status);
}
```

### 2. `while` and `for` Loops
```agam
fn main() {
    // Standard while loop
    let mut count = 0;
    while count < 5 {
        println("Count: " + count.to_string());
        count = count + 1;
    }

    // Range-based for loop
    for i in 0..5 {
        println("Iteration: " + i.to_string());
    }
}
```

---

## 20.2 Composite Structures (`struct`)

Structs group related data fields into custom types:

```agam
struct User {
    username: String,
    email: String,
    age: Int,
    is_active: Bool,
}

// Associated methods implementation block
impl User {
    fn new(name: String, email: String, age: Int) -> User {
        return User {
            username: name,
            email: email,
            age: age,
            is_active: true,
        };
    }

    fn deactivate(self) -> User {
        return User {
            username: self.username,
            email: self.email,
            age: self.age,
            is_active: false,
        };
    }
}

fn main() {
    let user1 = User.new("Alice", "alice@example.com", 28);
    println("User: " + user1.username);
}
```

---

## 20.3 Arrays & Tuples

```agam
fn main() {
    // Fixed-size homogeneous Array
    let numbers: Array[Int] = [10, 20, 30, 40, 50];
    println("First element: " + numbers[0].to_string());

    // Heterogeneous Tuple
    let pair: (String, Int) = ("Score", 99);
    println("Label: " + pair.0 + ", Value: " + pair.1.to_string());
}
```
