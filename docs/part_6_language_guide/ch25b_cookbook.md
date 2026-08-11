# Chapter 25b: Real-World Agam Code Cookbook

> **Part VI: The Agam Language Programming Guide**  
> **Target Audience**: Software Engineers building production applications in Agam

---

## Recipe 1: Production Web API Handler using Algebraic Effects

This recipe builds an HTTP API handler where database queries and logging side-effects are cleanly decoupled via algebraic effect handlers:

```agam
// 1. Declare Effect Interfaces
effect Database {
    fn find_user_by_id(id: Int) -> Option[String];
}

effect Logger {
    fn info(msg: String) -> Nil;
}

// 2. Pure Business Logic Function
fn handle_user_request(user_id: Int) -> String {
    perform Logger.info("Received API request for user ID: " + user_id.to_string());
    
    match perform Database.find_user_by_id(user_id) {
        Option.Some(user_json) => {
            perform Logger.info("User successfully found.");
            return "{ \"status\": 200, \"data\": " + user_json + " }";
        },
        Option.None => {
            perform Logger.info("User ID not found in database.");
            return "{ \"status\": 404, \"error\": \"User Not Found\" }";
        }
    }
}

// 3. Application Entrypoint with Concrete Handlers
fn main() {
    println("--- Test 1: Existing User ---");
    handle handle_user_request(42) {
        Logger.info(msg) => {
            println("[LOG]: " + msg);
            resume();
        },
        Database.find_user_by_id(id) => {
            if id == 42 {
                resume(Option.Some("{ \"name\": \"Alice\", \"role\": \"Admin\" }"));
            } else {
                resume(Option.None);
            }
        }
    }
}
```

---

## Recipe 2: Machine Learning Tensor Training Pipeline

This recipe constructs a 2-layer neural network forward pass using Agam's native tensors:

```agam
struct MultiLayerPerceptron {
    w1: Tensor[Float],
    b1: Tensor[Float],
    w2: Tensor[Float],
    b2: Tensor[Float],
}

impl MultiLayerPerceptron {
    fn new(in_dim: Int, hidden_dim: Int, out_dim: Int) -> MultiLayerPerceptron {
        return MultiLayerPerceptron {
            w1: Tensor.random([in_dim, hidden_dim]),
            b1: Tensor.zeros([hidden_dim]),
            w2: Tensor.random([hidden_dim, out_dim]),
            b2: Tensor.zeros([out_dim]),
        };
    }

    fn forward(self, x: Tensor[Float]) -> Tensor[Float] {
        // Layer 1: Linear + ReLU
        let h1 = Tensor.relu((x * self.w1) + self.b1);
        // Layer 2: Linear Output
        let out = (h1 * self.w2) + self.b2;
        return out;
    }
}

fn main() {
    let mlp = MultiLayerPerceptron.new(784, 128, 10);
    let sample_batch = Tensor.ones([32, 784]); // Batch size 32, 784 features
    
    let predictions = mlp.forward(sample_batch);
    println("Output Batch Tensor Shape: " + predictions.shape().to_string());
}
```
