#!/usr/bin/env python3
"""
Test runner script for RequestX following CI/CD pipeline structure.

This script runs the complete test suite in the order specified by the CI/CD pipeline:
1. Unit tests for core functionality
2. Integration tests for requests compatibility
3. Coverage measurement and reporting

Requirements: 6.1, 7.1, 7.2, 7.3, 7.4
"""

import unittest
import sys
import os
import time
import subprocess
from pathlib import Path

# Add the parent directory to the path to import requestx
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "python"))

try:
    import requestx

    print(
        f"✓ RequestX imported successfully (version: {getattr(requestx, '__version__', 'unknown')})"
    )
except ImportError as e:
    print(f"✗ Failed to import requestx: {e}")
    print("Make sure to build the extension with: uv run maturin develop")
    sys.exit(1)


class TestRunner:
    """Test runner that follows CI/CD pipeline structure."""

    def __init__(self):
        self.test_dir = Path(__file__).parent
        self.results = {}
        self.total_tests = 0
        self.total_failures = 0
        self.total_errors = 0

    def run_test_module(self, module_name, description):
        """Run a specific test module and collect results."""
        print(f"\n{'='*60}")
        print(f"Running {description}")
        print(f"{'='*60}")

        try:
            # Import the test module
            module_path = self.test_dir / f"{module_name}.py"
            if not module_path.exists():
                print(f"⚠ Test module {module_name}.py not found, skipping...")
                return True

            # Add test directory to path for imports
            if str(self.test_dir) not in sys.path:
                sys.path.insert(0, str(self.test_dir))

            test_module = __import__(module_name)

            # Create test suite from the module
            loader = unittest.TestLoader()
            suite = loader.loadTestsFromModule(test_module)

            # Run the tests
            runner = unittest.TextTestRunner(verbosity=2, stream=sys.stdout)
            result = runner.run(suite)

            # Collect results
            self.results[module_name] = {
                "description": description,
                "tests_run": result.testsRun,
                "failures": len(result.failures),
                "errors": len(result.errors),
                "success": result.wasSuccessful(),
            }

            self.total_tests += result.testsRun
            self.total_failures += len(result.failures)
            self.total_errors += len(result.errors)

            if result.wasSuccessful():
                print(f"✓ {description} - All tests passed!")
            else:
                print(
                    f"✗ {description} - {len(result.failures)} failures, {len(result.errors)} errors"
                )

            return result.wasSuccessful()

        except Exception as e:
            print(f"✗ Failed to run {description}: {e}")
            self.results[module_name] = {
                "description": description,
                "tests_run": 0,
                "failures": 0,
                "errors": 1,
                "success": False,
                "error": str(e),
            }
            self.total_errors += 1
            return False

    def run_async_tests(self, module_name, description):
        """Run async tests using the module's async test runner."""
        print(f"\n{'='*60}")
        print(f"Running {description}")
        print(f"{'='*60}")

        try:
            # Import and run async tests
            module_path = self.test_dir / f"{module_name}.py"
            if not module_path.exists():
                print(f"⚠ Test module {module_name}.py not found, skipping...")
                return True

            # Add test directory to path for imports
            if str(self.test_dir) not in sys.path:
                sys.path.insert(0, str(self.test_dir))

            test_module = __import__(module_name)

            if hasattr(test_module, "run_async_tests"):
                test_module.run_async_tests()
                print(f"✓ {description} - All async tests passed!")

                self.results[f"{module_name}_async"] = {
                    "description": f"{description} (Async)",
                    "tests_run": "N/A",
                    "failures": 0,
                    "errors": 0,
                    "success": True,
                }
                return True
            else:
                print(f"⚠ {description} - No async test runner found")
                return True

        except Exception as e:
            print(f"✗ Failed to run {description}: {e}")
            self.results[f"{module_name}_async"] = {
                "description": f"{description} (Async)",
                "tests_run": "N/A",
                "failures": 0,
                "errors": 1,
                "success": False,
                "error": str(e),
            }
            self.total_errors += 1
            return False

    def run_coverage_analysis(self):
        """Run coverage analysis if coverage tools are available."""
        print(f"\n{'='*60}")
        print("Running Coverage Analysis")
        print(f"{'='*60}")

        try:
            # Try to import coverage
            import coverage

            # Create coverage instance
            cov = coverage.Coverage()
            cov.start()

            # Re-run core tests with coverage
            print("Re-running core tests with coverage measurement...")

            # Import and run key test modules
            test_modules = ["test_unit", "test_requests_compatibility"]

            for module_name in test_modules:
                try:
                    # Add test directory to path for imports
                    test_dir = Path(__file__).parent
                    if str(test_dir) not in sys.path:
                        sys.path.insert(0, str(test_dir))

                    test_module = __import__(module_name)
                    loader = unittest.TestLoader()
                    suite = loader.loadTestsFromModule(test_module)
                    runner = unittest.TextTestRunner(
                        verbosity=0, stream=open(os.devnull, "w")
                    )
                    runner.run(suite)
                except Exception as e:
                    print(f"Warning: Could not run {module_name} for coverage: {e}")

            # Stop coverage and generate report
            cov.stop()
            cov.save()

            print("\nCoverage Report:")
            cov.report(show_missing=True)

            # Try to generate HTML report
            try:
                cov.html_report(directory="htmlcov")
                print("HTML coverage report generated in 'htmlcov/' directory")
            except Exception:
                pass

            return True

        except ImportError:
            print("⚠ Coverage tools not available. Install with: pip install coverage")
            print("Skipping coverage analysis...")
            return True
        except Exception as e:
            print(f"✗ Coverage analysis failed: {e}")
            return False

    def print_summary(self):
        """Print test summary."""
        print(f"\n{'='*60}")
        print("TEST SUMMARY")
        print(f"{'='*60}")

        all_passed = True

        for module, result in self.results.items():
            status = "✓ PASSED" if result["success"] else "✗ FAILED"
            print(f"{status:10} {result['description']}")

            if not result["success"]:
                all_passed = False
                if "error" in result:
                    print(f"           Error: {result['error']}")
                else:
                    print(
                        f"           Tests: {result['tests_run']}, "
                        f"Failures: {result['failures']}, "
                        f"Errors: {result['errors']}"
                    )

        print(f"\nOverall Results:")
        print(f"Total Tests Run: {self.total_tests}")
        print(f"Total Failures: {self.total_failures}")
        print(f"Total Errors: {self.total_errors}")
        print(
            f"Success Rate: {((self.total_tests - self.total_failures - self.total_errors) / max(self.total_tests, 1)) * 100:.1f}%"
        )

        if all_passed:
            print(f"\n🎉 ALL TESTS PASSED! 🎉")
        else:
            print(f"\n❌ SOME TESTS FAILED ❌")

        return all_passed

    def run_all_tests(self):
        """Run all tests following CI/CD pipeline order."""
        print("RequestX Test Suite")
        print("Following CI/CD Pipeline Structure")
        print(f"Python version: {sys.version}")
        print(f"Test directory: {self.test_dir}")

        start_time = time.time()

        # Stage 1: Core Unit Tests
        success = True
        success &= self.run_test_module("test_unit", "Core Unit Tests")

        # Stage 2: Comprehensive Tests
        success &= self.run_test_module(
            "test_comprehensive", "Comprehensive HTTP Tests"
        )

        # Stage 3: Requests Compatibility Tests
        success &= self.run_test_module(
            "test_requests_compatibility", "Requests Compatibility Tests"
        )

        # Stage 4: Async Runtime Tests
        success &= self.run_test_module("test_async_runtime", "Async Runtime Tests")
        success &= self.run_async_tests("test_async_runtime", "Async Runtime Tests")

        # Stage 5: Error Handling Tests
        success &= self.run_test_module("test_error_handling", "Error Handling Tests")

        # Stage 6: Response Object Tests
        success &= self.run_test_module("test_response", "Response Object Tests")

        # Stage 7: Session Management Tests
        success &= self.run_test_module("test_session", "Session Management Tests")

        # Stage 8: Performance Tests (if available)
        if (self.test_dir / "test_performance.py").exists():
            success &= self.run_test_module("test_performance", "Performance Tests")

        # Stage 9: Coverage Analysis
        self.run_coverage_analysis()

        end_time = time.time()

        # Print final summary
        self.print_summary()
        print(f"\nTotal execution time: {end_time - start_time:.2f} seconds")

        return success


def main():
    """Main entry point."""
    runner = TestRunner()
    success = runner.run_all_tests()

    if success:
        sys.exit(0)
    else:
        sys.exit(1)


if __name__ == "__main__":
    main()
