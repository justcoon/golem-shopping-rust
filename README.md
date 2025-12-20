# golem-shopping-rust

## Overview
This is a simple e-commerce application built with an agent-based architecture, leveraging Golem Cloud for distributed, stateful agents. The application provides a complete online shopping experience with product browsing, cart management, order processing, and an AI-powered shopping assistant.

![Architecture Diagram](architecture.png)

*Figure 1: Golem Shopping Application Architecture*

## Architecture Components

### Agents
- **Product Agent**: Manages the product catalog and product-related operations.
- **Pricing Agent**: Handles product pricing, discounts, and promotions.
- **Cart Agent**: Manages user shopping carts, with one cart per user, and handles cart-to-order conversion.
- **Order Agent**: Manages the complete order lifecycle from creation to fulfillment.
- **Product Search Agent**: Provides product search capabilities across the catalog.
- **Shopping Assistant**: AI-powered assistant that helps users find products and manage their shopping experience.

### Key Features
- **RESTful API** for all shopping operations
- **Stateful Agents** with Golem Cloud managing the state
- **AI Integration** with external LLM service for the shopping assistant
- **Distributed Agent System** with clear responsibility boundaries

### Communication Flow
1. Users interact with the system through the API Gateway
2. The gateway routes requests to the appropriate agents
3. Agents communicate via RPC calls as needed
4. External AI/LLM service enhance the Shopping Assistant's capabilities

### State Management
All core agents (Product, Pricing, Cart, Order, and Shopping Assistant) have their state managed by Golem Cloud, ensuring reliability and scalability through the agent-based architecture.


## Quick Start

1. **Prerequisites**:
    - Install [Golem CLI](https://learn.golem.cloud/cli) (version 1.4.0+)
    - [Running Golem Environment](https://learn.golem.cloud/quickstart#running-golem)

   See [Golem Quickstart](https://learn.golem.cloud/quickstart) for more information.


2. **Build and Deploy**:
   ```bash
   # Build all components
   golem-cli build
   
   # Deploy to Golem
   golem-cli deploy
   ```
NOTE: shopping-assistant agent using Open Router API for LLM calls. API key is required to run the app (`OPENROUTER_KEY` environment variable)

3. **Import Sample Data**:
   For information on importing sample data, see the [Data README](./data/README.md).

