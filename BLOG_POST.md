# Building a Distributed Shopping Application with Rust and Golem: An Agent-Based Architecture

## Introduction

In today's cloud-native world, developers are constantly seeking more efficient and scalable ways to build applications. The Golem Shopping project demonstrates how to build a distributed shopping application using Rust and the [Golem Cloud](https://golem.cloud/), showcasing the power of WebAssembly (Wasm) and agent-native architectures.

## Project Overview

Golem Shopping is a modular e-commerce application composed of four main components:

1. **Product Agent**: Manages product information
2. **Pricing Agent**: Handles product pricing
3. **Cart Agent**: Manages user shopping carts
4. **Order Agent**: Processes and tracks orders
5. **Product Search Agent**: Handles product search functionality
6. **Shopping Assistant Agent**: AI-powered assistant for personalized shopping experiences

## Technical Architecture

### Built with Rust and WebAssembly

The entire application is written in Rust and compiled to WebAssembly, offering near-native performance with the safety guarantees of Rust's ownership model. Each component is deployed as an independent Golem agent, communicating through well-defined interfaces.

### Key Technologies

- **Rust**: For type-safe, performant code
- **WebAssembly (Wasm)**: For portable, secure execution
- **Golem Cloud**: For distributed computation

### Architecture Overview

The following diagram illustrates the high-level architecture of the Golem Shopping application:

![Golem Shopping Architecture](architecture.png)

*Figure 1: Golem Shopping Application Architecture*

### Communication Flow

1. Users interact with the system through the API Gateway
2. The gateway routes requests to the appropriate agents
3. Agents communicate via RPC calls as needed
4. External AI/LLM service enhances the Shopping Assistant's capabilities

## Component Design

### 1. Product Agent

The Product Agent serves as the authoritative source for product information. By assigning a dedicated agent to each product, the system achieves fine-grained isolation and scalability. This agent-based approach allows individual products to be updated largely independently, ensuring that high-traffic items don't impact the performance of the rest of the catalog.

```rust
#[agent_definition]
trait ProductAgent {
    fn new(id: String) -> Self;

    fn get_product(&self) -> Option<Product>;

    fn initialize_product(
        &mut self,
        name: String,
        brand: String,
        description: String,
        tags: Vec<String>,
    );
}
```

### 2. Pricing Agent

Complementing the product catalog, the Pricing Agent encapsulates all pricing logic. Separating pricing from product data allows for dynamic strategies—such as discounts, flash sales, or personalized offers—to be deployed without modifying the core product definitions. This separation of concerns enables the business to iterate on pricing models rapidly with zero downtime.

```rust
#[agent_definition]
trait PricingAgent {
    fn new(id: String) -> Self;

    fn get_pricing(&self) -> Option<Pricing>;

    fn get_price(&self, currency: String, zone: String) -> Option<PricingItem>;

    fn initialize_pricing(
        &mut self,
        msrp_prices: Vec<PricingItem>,
        list_prices: Vec<PricingItem>,
        sale_prices: Vec<SalePricingItem>,
    );

    fn update_pricing(
        &mut self,
        msrp_prices: Vec<PricingItem>,
        list_prices: Vec<PricingItem>,
        sale_prices: Vec<SalePricingItem>,
    );
}
```

### 3. Cart Agent

The Cart Agent anchors the user's shopping experience by providing a persistent, individual shopping cart. Maintained as a stateful entity for every user, it handles the addition and removal of items while performing real-time price validation. When a user is ready to buy, the Cart Agent seamlessly hands off the session data to the Order Agent, ensuring a smooth transition from browsing to purchasing.

```rust
#[agent_definition]
trait CartAgent {
    fn new(id: String) -> Self;
    async fn get_cart(&mut self) -> Option<Cart>;
    async fn add_item(&mut self, product_id: String, quantity: u32) -> Result<(), AddItemError>;
    async fn checkout(&mut self) -> Result<OrderConfirmation, CheckoutError>;
    fn update_email(&mut self, email: String) -> Result<(), UpdateEmailError>;
    fn clear(&mut self);
    fn remove_item(&mut self, product_id: String) -> Result<(), RemoveItemError>;
    fn update_billing_address(&mut self, address: Address) -> Result<(), UpdateAddressError>;
    fn update_item_quantity(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<(), UpdateItemQuantityError>;
    fn update_shipping_address(&mut self, address: Address) -> Result<(), UpdateAddressError>;
}
```

The `get_cart` method showcases the power of agent composition. It enriches the cart by fetching fresh product details and pricing information in parallel from the Product and Pricing agents. This ensures that the user always sees the most up-to-date information without the Cart agent needing to duplicate this state.

```rust
    async fn get_cart(&mut self) -> Option<Cart> {
        println!("Getting cart");
        if let Some(cart) = self.state.as_mut() {
            let mut items = Vec::new();
            for item in cart.items.clone() {
                let product_id = item.product_id;
                let quantity = item.quantity;

                let product_client = ProductAgentClient::get(product_id.clone());
                let pricing_client = PricingAgentClient::get(product_id.clone());

                // Fetch product and pricing in parallel
                let (product, pricing) = join(
                    product_client.get_product(),
                    pricing_client
                        .get_price(cart.currency.clone(), PRICING_ZONE_DEFAULT.to_string()),
                )
                .await;

                if let (Some(product), Some(pricing)) = (product, pricing) {
                    items.push(get_cart_item(product, pricing, quantity));
                }
            }
            cart.set_items(items);
            Some(cart.clone())
        } else {
            None
        }
    }
```

The following snippet demonstrates the implementation of the checkout process, where the Cart Agent orchestrates the order creation and triggers the Shopping Assistant:

```rust
    async fn checkout(&mut self) -> Result<OrderConfirmation, CheckoutError> {
        let state = self.get_state();
        let order_id = generate_order_id();
        println!("Checkout for order {}", order_id);

        create_order(order_id.clone(), state.clone()).await?;

        state.order_created(order_id.clone());

        ShoppingAssistantAgentClient::get(state.user_id.clone()).trigger_recommend_items();

        Ok(OrderConfirmation { order_id })
    }
```

### 4. Product Search Agent

Unlike its stateful counterparts, the Product Search Agent is designed for high throughput and stateless operation. It acts as an intelligent router, querying multiple product agents to aggregate results for user searches. Because it maintains no persistent state of its own, it can be scaled horizontally with ease to handle spikes in search traffic.

```rust
#[agent_definition(mode = "ephemeral")]
trait ProductSearchAgent {
    fn new() -> Self;

    async fn search(&self, query: String) -> Result<Vec<Product>, String>;
}
```

### 5. Order Agent

Once a purchase is committed, the Order Agent takes over to manage the lifecycle of the transaction. It acts as the guardian of order integrity, enforcing valid state transitions from creation to fulfillment. By strictly managing states—such as 'New', 'Shipped', or 'Cancelled'—it ensures that orders become immutable once fulfilled, preserving a reliable audit trail of the business's history.

```rust
#[agent_definition]
trait OrderAgent {
    fn new(id: String) -> Self;
    fn initialize_order(&mut self, data: CreateOrder) -> Result<(), InitOrderError>;
    fn get_order(&self) -> Option<Order>;
    async fn add_item(&mut self, product_id: String, quantity: u32) -> Result<(), AddItemError>;
    fn update_email(&mut self, email: String) -> Result<(), UpdateEmailError>;
    fn remove_item(&mut self, product_id: String) -> Result<(), RemoveItemError>;
    fn update_billing_address(&mut self, address: Address) -> Result<(), UpdateAddressError>;
    fn update_item_quantity(
        &mut self,
        product_id: String,
        quantity: u32,
    ) -> Result<(), UpdateItemQuantityError>;
    fn update_shipping_address(&mut self, address: Address) -> Result<(), UpdateAddressError>;
    fn ship_order(&mut self) -> Result<(), ShipOrderError>;
    fn cancel_order(&mut self) -> Result<(), CancelOrderError>;
}
```

### 6. Shopping Assistant Agent

Finally, the Shopping Assistant bridges the gap between deterministic business logic and probabilistic AI. It is context-aware, using the user's shopping history to make intelligent recommendations for specific products and related brands.

```rust
#[agent_definition]
trait ShoppingAssistantAgent {
    fn new(id: String) -> Self;

    fn get_recommended_items(&self) -> RecommendedItems;

    async fn recommend_items(&mut self) -> bool;
}
```

The `recommend_items` function gathers the user's order history and sends it to an LLM to generate personalized product and brand recommendations. This illustrates how Golem agents can seamlessly integrate external AI services into stateful workflows.

```rust
    async fn recommend_items(&mut self) -> bool {
        let order_items = get_order_items(self._id.clone()).await;
        // Integrating with an LLM for recommendations
        let recommended_items = get_llm_recommendations(order_items).await;

        match recommended_items {
            Ok(recommended_items) => {
                self.recommended_items = RecommendedItems {
                    product_ids: recommended_items.product_ids,
                    product_brands: recommended_items.product_brands,
                    updated_at: chrono::Utc::now(),
                };
                true
            }
            Err(e) => {
                println!("Recommended items - error: {}", e);
                false
            }
        }
    }
```

## Key Features

### 1. Durable Agents

Golem provides automatic state persistence, ensuring that all code executed within the platform is 100% durable. Unlike traditional frameworks that often require complex DSLs or external databases to manage state, Golem allows developers to write standard code while the platform handles persistence transparently. This means every variable and in-memory structure is automatically saved and restored, simplifying development and eliminating widespread classes of reliability bugs.

### 2. Agent-to-Agent Communication

The Product Search Agent demonstrates efficient service decomposition by:
- Delegating data storage to the Product Agent
- Focusing solely on search request routing and response aggregation
- Enabling independent scaling of search functionality

Components communicate using Golem's RPC mechanism, enabling:

- Loose coupling between agents
- Location transparency
- Exactly-once agent-to-agent communication

### 3. REST API Gateway

The application exposes REST APIs through Golem's API gateway, providing:

- Standard HTTP interfaces
- Easy integration with web and mobile clients

## Getting Started

### Prerequisites

- Rust toolchain
- Golem CLI
- Docker (for local development)

### Building and Deploying

```bash
# Build all components
golem-cli build

# Deploy to Golem
golem-cli deploy
```

### Interacting with the Services

```bash
golem-cli repl
```

## Performance Benchmarks

To ensure the Golem Shopping application meets production-grade performance requirements, we've conducted extensive load testing using the Goose load testing framework. These benchmarks demonstrate the system's ability to handle real-world e-commerce traffic patterns.

### Test Environment

- **Hardware**: Local development environment (MacBook Pro 2019, 2,4 GHz 8-Core Intel Core i9, 32 GB RAM) with Golem [running locally in Docker](https://github.com/golemcloud/golem/tree/main/docker-examples/published-postgres)
- **Concurrent Users**: 16 virtual users
- **Test Duration**: Approximately 3 minutes
- **Test Scenarios**:
  1. **Product Lookup**: Retrieve product details
  2. **Pricing Lookup**: Fetch product pricing
  3. **Product Search By Brand**: Perform product searches
  4. **Cart Operations**: Complete cart workflow including:
     - Adding items to cart
     - Removing items
     - Setting email
     - Setting billing address
     - Checking out
     - Retrieving order details

### Key Performance Metrics

| Operation                           | Average Response Time | Requests per Second |
|-------------------------------------|-----------------------|---------------------|
| Get Product                         | 31ms                  | 0.42 RPS            |
| Get Pricing                         | 34ms                  | 0.39 RPS            |
| Product Search By Brand             | 2100ms                | 0.34 RPS            |
| Create, checkout Cart and get Order | 170ms                 | 0.32 RPS            |

### Test Data

- **Products**: 50 unique products (IDs: p001-p050)
- **Users**: 10 unique user sessions (user001-user010)
- **Cart Items**: 4 items per cart on average

### Performance Characteristics

1. **Consistent Latency**: The system maintains sub-100ms response times for core read operations (Product, Pricing) even under load.
2. **High Throughput**: The application handles approximately 4.4 requests per second across all endpoints in this local configuration.
3. **Reliability**: 100% success rate across all test scenarios, demonstrating the system's stability.
4. **Scalability**: The agent-based architecture allows horizontal scaling of individual components based on demand.

### Benchmark Execution

Benchmarks can be reproduced using the following commands:

```bash
# Set environment variables
export HOST=http://localhost:9006
export API_HOST=http://localhost:9006

# Run benchmarks
cargo run --release -- --report-file=report.html --no-reset-metrics
```

See [benchmarks/README.md](https://github.com/justcoon/golem-shopping/blob/main/benchmarks/README.md) for more details

## Benefits of This Architecture

1. **Scalability**: Each component scales independently based on demand
2. **Resilience**: Isolated failures don't bring down the entire system
3. **Developer Experience**: Clear boundaries between agents
4. **Cost Efficiency**: Pay only for the compute you use

## Real-World Applications

The patterns demonstrated in this project can be applied to:

- Agent-based architectures
- Microservices architectures
- Agent-native applications
- Distributed systems
- E-commerce platforms

## Conclusion

The Golem Shopping project showcases how modern web technologies like Rust, WebAssembly, and the Golem Cloud can be combined to build scalable, maintainable distributed applications. By leveraging these technologies, developers can create systems that are both performant and easy to reason about.

## Next Steps

1. Explore the [GitHub repository](https://github.com/justcoon/golem-shopping-rust)
2. Try deploying your own instance
3. Contribute to the project
4. Check out the [TypeScript implementation](https://github.com/justcoon/golem-shopping-ts) for a similar application built with TypeScript

## Resources

- [Golem Documentation](https://learn.golem.cloud/)
- [Rust Programming Language](https://www.rust-lang.org/)
- [WebAssembly](https://webassembly.org/)
