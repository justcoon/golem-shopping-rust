# Sample Data

This directory contains sample data and import scripts for the Golem Shopping application.

## Data Files

- `fixtures/tech_products.csv`: Sample product data with fields like product ID, name, and brand
- `fixtures/tech_prices.csv`: Sample pricing data for the products
- `import.yaml`: Configuration for importing the sample data using the Drill framework

## Importing Sample Data

### Prerequisites

1. Install [drill](https://github.com/fcsonline/drill):
   ```bash
   cargo install drill
   ```

2. Ensure the Golem Shopping application is running

### Environment Variables

- `HOST`: Worker service API gateway host (e.g., `http://localhost:9006`)
- `API_HOST`: API deployment host/site (e.g., `http://localhost:9006`)

### Running the Import

From the `data` directory, run:

```bash
HOST=http://localhost:9006 API_HOST=localhost:9006 drill --benchmark import.yaml --stats
```

This will import all sample products and their corresponding prices into your Golem Shopping application.

## Data Structure

### Products (`tech_products.csv`)
- `product-id`: Unique identifier for the product
- `body`: JSON string containing product details (name, description, etc.)

### Prices (`tech_prices.csv`)
- `product-id`: References a product from the products file
- `body`: JSON string containing pricing information

## Troubleshooting

- Ensure both the API gateway and the application are running before importing data
- Verify that the environment variables are correctly set
- Check the console output for any error messages during import