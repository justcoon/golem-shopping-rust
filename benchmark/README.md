# Performance Benchmarks

This directory contains benchmarking tools and configurations for testing the performance of the Golem Shopping application.

## Benchmarking Tools

- **[Goose](https://github.com/tag1consulting/goose)**: Used for load testing the application
- **[Drill](https://github.com/fcsonline/drill)**: Used for importing test data

## Prerequisites

1. Install required tools:
   ```bash
   # Install Goose
   cargo install cargo-goose
   
   # Install Drill
   cargo install drill
   ```

2. Ensure the Golem Shopping application is running

## Environment Variables

- `HOST`: Worker service API gateway host (e.g., `http://localhost:9006`)
- `API_HOST`: API deployment host/site (e.g., `http://localhost:9006`)

## Importing Test Data

Before running benchmarks, you'll need to import test data using Drill:

```bash
cd benchmark
HOST=http://localhost:9006 API_HOST=localhost:9006 drill --benchmark import.yaml --stats
```

This will import:
- Products with IDs: p001 - p049
- Pricing for product IDs: p001 - p050

## Running Benchmarks

To run the load tests:

```bash
cd benchmark
HOST=http://localhost:9006 API_HOST=localhost:9006 cargo run --release -- --report-file=report.html --no-reset-metrics
```

### Test Coverage

The benchmark tests the following components:
- Products (IDs: p001 - p050)
- Pricing (IDs: p001 - p050)
- Carts (User IDs: user001 - user010)

> **Note**: Some requests for p050 may produce errors as part of the test scenario.

## Understanding the Results

After running the benchmarks, a report will be generated at `report.html`. This report includes:

- Response times (min, max, average)
- Requests per second
- Error rates
- Detailed metrics for each endpoint

## Customizing Benchmarks

You can customize the benchmark parameters by modifying:

- `src/main.rs`: Adjust test scenarios and load patterns
- `import.yaml`: Modify the test data import configuration
- `fixtures/`: Contains sample data files for testing

## Troubleshooting

- Ensure all services are running before starting benchmarks
- Verify environment variables are correctly set
- Check for port conflicts if benchmarks fail to start
- Review the generated report for detailed error information