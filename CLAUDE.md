# Lightweight mesh preprocessing application

The PLAN.md file includes a list of objectives I wish to implement.
Each task should be performed on a separate branch.
Before creating a new branch, ensure that all changes on `main` have been pulled.

After implementing changes as outlined in the PLAN.md file, always review the PLAN.md contents and check off items that have been completed.

Do not create summary markdown files after a task has been completed unless otherwise instructed.

# Demo Scripts

The `./run_demo.sh` script can be run as a quick test to demonstrate the end to end features.

## Example Exodus Files

The ./test-data directory contains exodus files.
Some of the exodus files containe sidesets and nodesets, and some do not.
The exodus files that are missing sidesets and nodesets can be used for the fully automated contact surface detection pipeline.
The exodus files that include sidesets can be used for contact surface matching or pairing without first needing to identify the sidesets.

# Testing

Tests are located in the ./tests directory.
