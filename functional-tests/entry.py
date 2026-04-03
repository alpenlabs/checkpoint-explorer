#!/usr/bin/env python3
"""Entry point for checkpoint-explorer functional tests."""

import argparse
import os
import sys

import flexitest

from envs.testenv import ExplorerEnvConfig

TEST_DIR = "tests"
DD_ROOT = ".dd"


def main(argv):
    parser = argparse.ArgumentParser(prog="entry.py")
    parser.add_argument("-t", "--tests", nargs="*", help="Run specific test(s) by name")
    parser.add_argument("--list-tests", action="store_true", help="List all available tests")
    parsed = parser.parse_args(argv[1:])

    root_dir = os.path.dirname(os.path.abspath(__file__))
    test_dir = os.path.join(root_dir, TEST_DIR)

    modules = {
        k: v
        for k, v in flexitest.runtime.scan_dir_for_modules(test_dir).items()
        if k != "__init__"
    }

    if parsed.list_tests:
        for name in sorted(modules):
            print(name)
        return 0

    if parsed.tests:
        arg_tests = frozenset(parsed.tests)
        modules = {k: v for k, v in modules.items() if k in arg_tests}

    flexitest.runtime.load_candidate_modules(modules)

    global_envs = {"explorer": ExplorerEnvConfig()}

    datadir_root = flexitest.create_datadir_in_workspace(os.path.join(root_dir, DD_ROOT))
    rt = flexitest.TestRuntime(global_envs, datadir_root, {})
    rt.prepare_registered_tests()

    test_names = list(rt.tests.keys())
    results = rt.run_tests(test_names)
    flexitest.dump_results(results)
    flexitest.fail_on_error(results)

    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
