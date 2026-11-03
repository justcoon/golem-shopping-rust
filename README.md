# golem-shopping-rust

## Overview
This is a modern e-commerce application built with an agent-based architecture, leveraging Golem Cloud for distributed, stateful WebAssembly agents. The application provides an end-to-end online shopping experience featuring product browsing, inventory reservations, cart management, order lifecycle handling, an AI-powered conversational shopping assistant, and an operations store manager.

![Architecture Diagram](architecture.png)

*Figure 1: Golem Shopping Application Architecture*

## Architecture Components

### Agents

#### Commerce Agents (Stateful)
- **Product Agent**: Manages individual product catalog details, attributes, and descriptions (`/v1/product/{id}`).
- **Pricing Agent**: Handles product base pricing, regional discounts, and promotional rules (`/v1/pricing/{id}`).
- **Inventory Agent**: Manages regional product stock levels, reservation allocations for pending orders, and inventory adjustments/fulfillments (`/v1/inventory/{id}`).
- **Cart Agent**: Manages user shopping carts, cart-to-order checkout conversion, saved contact details, and past order history (`/v1/cart/{id}`).
- **Order Agent**: Manages the complete order lifecycle from creation through fulfillment, status updates, and inventory synchronization (`/v1/order/{id}`).

#### Search Agents (Ephemeral)
- **Product Search Agent**: Provides catalog-wide search, filtering, and batch lookups enriched with live prices and stock availability (`/v1/product/search`).
- **Order Search Agent**: Provides cross-customer order search, filtering, and discovery by status or date range (`/v1/order-search/**`).

#### AI Assistants (Two-Tier Multi-Thread Architecture)
- **Shopping Assistant Workspace Agent (`ShoppingAssistantAgent`)**: Per-user workspace agent (`/v1/assistant/{user_id}`). Manages the catalog of conversation threads (`/chats`), user-level cached recommendations (`/recommended-items`), and backwards-compatible proxies.
- **Shopping Assistant Chat Agent (`ShoppingAssistantChatAgent`)**: Discrete conversation thread actor (`/v1/assistant/{user_id}/chats/{chat_id}`). Equipped with 16 domain tools for product search, price lookup, stock availability checking, cart manipulation, checkout, and order status inquiry. Supports real-time bi-directional streaming via **Golem Durable Streams** (`/chat-session`) and non-streaming (`/chat`) without blocking the parent assistant or other threads.
- **Manager Workspace Agent (`ManagerAgent`)**: Per-manager workspace agent (`/v1/manager/{manager_id}`). Manages the list of operational threads (`/chats`) and store management state.
- **Manager Chat Agent (`ManagerChatAgent`)**: Discrete operational thread actor (`/v1/manager/{manager_id}/chats/{chat_id}`). Enables store managers to query order histories, update order statuses, inspect and adjust inventory levels, and update product pricing using conversational AI and Golem Durable Streams.

#### Static Web Agents (Ephemeral)
- **Web Assistant & Web Manager**: Lightweight WASM agents serving the embedded HTML/JS web chat interfaces from Golem's virtual filesystem (`/assistant` and `/manager`).

### Frontend Applications
- **Vue 3 SPA (`frontend/`)**: Modern Single Page Application built with Vue 3, TypeScript, Pinia, Tailwind CSS, and Vite. Implements full product browsing, cart management, checkout, order history, and an interactive chat assistant with Golem Durable Streams support. Strictly conforms to `snake_case` API contracts.
- **Embedded Web Chat (`web/`)**: Self-contained static web interfaces for the Shopping Assistant and Store Manager, served directly by Golem agents.

### Key Features
- **RESTful API & Streaming**: Standard REST API for shopping operations combined with bi-directional streaming powered by **Golem Durable Streams** (`inbox`/`outbox` slots) for conversational AI.
- **Durable Execution & State Persistence**: All stateful agents automatically persist state and execution history across restarts and failures without requiring external database boilerplate.
- **AI Integration**: Powered by external LLM services (defaults to Google Gemini API `gemini-3.1-flash-lite` with thought signature preservation for tool calls; also supports OpenRouter).
- **Agent-to-Agent RPC**: Type-safe inter-agent communication across boundaries.

---

## Quick Start

### 1. Prerequisites
- **Rust** with `wasm32-wasip2` target:
  ```bash
  rustup target add wasm32-wasip2
  ```
- **Golem CLI** (version 1.6.0+): [Install Golem CLI](https://learn.golem.cloud/cli)
- **Node.js** (v18+) & **npm** (for the Vue 3 frontend)

### 2. Configure LLM API Keys
The AI assistants require an LLM API key. By default, `golem.yaml` is configured for Google Gemini API (`gemini-3.1-flash-lite`). You can provide either:

```bash
# Google Gemini API key (default)
export GEMINI_API_KEY="your-gemini-api-key"

# Or OpenRouter API key (if using OpenRouter config in golem.yaml)
export OPENROUTER_KEY="your-openrouter-key"
```

### 3. Build and Deploy Golem Backend

Start the local Golem dev server if not already running:
```bash
golem server
```

Build and deploy the application:
```bash
# Compile WASM components
golem build --yes

# Deploy agents and API routing
golem deploy --yes
```

The Golem API Gateway will be available at `http://localhost:9006`.

### 4. Run Frontend Applications

#### Option A: Modern Vue 3 Frontend
Refer to the [Frontend README](frontend/README.md) for setup and development server instructions (`http://localhost:3000`).

#### Option B: Embedded Web Chat Interfaces
Navigate directly to:
- Shopping Assistant: `http://localhost:9006/assistant`
- Store Manager: `http://localhost:9006/manager`

### 5. Import Sample Data
To populate the store with sample products, prices, and regional inventory fixtures, refer to the [Sample Data README](data/README.md) for instructions on using `drill` with `data/import.yaml`.
