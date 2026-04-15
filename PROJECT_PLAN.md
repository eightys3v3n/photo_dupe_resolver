# Overview
A program to resolve duplicate photos using scenarios that guide automated actions on photo collections.

# Current Project State
The web UI is functional.
The program can scan for images, hash them, create groups of similar images, and display all that in the web UI.
Next steps would be to come up with a sample image set to implement some scenarios and test them.

# How to use
- Run `cargo run` to start and open the Web UI.
- Optionally run the test cases with `cargo test`.
- Browse to the web UI.
- Start the scanner process.
- Start the hasher process.
- Wait for the scanner and hasher to finish.
- Use the duplicate group viewer area to browse duplicates and generate scenarios for what automated actions could be performed on those image groups.

# Architecture
## Configuration
Provides loading and access for the whole program to a configuration file. This stores both random config options like "db loading batch size" and scenarios used to automatically act on the photo collection.

## CLI
Can provide any optionsaside from scenarios as CLI arguments and they should override hte config file arguments. Can also specify the config file path.

## Web UI
Shows a web UI in a separate process that can display the status of the database (number of images discovered, hashing progress, hashing speed, can show all images discovered with search functionality).
The web UI should access a global state that all the other main threads can update. The web UI should reload when the state updates.
It doesn't have to look amazing but it has to be functional.
Has pages to manage the process like starting / stopping the scanner, adding more paths to the scanner, starting / stopping the hasher.
There should be a separate tab in the web UI for controlling and seeing the status of the process (start/stop, found files count, etc).
There should be a separate tab in the web UI for seeing and searching a paginated list of all discovered images.
There should be a separate tab in the web UI for seeing and searching image duplicate groups with little thumbnails.

## Database
Provides querying capabilities for a database backend (SQLite to start).
It shouldn't allow direct querying of the database, instead helper methods must be used (like get_all_images to list all images in the DB for example).
It should automatically setup the database schema when it doesn't exist.
It should perform sane recovery on a database state that isn't expected (never losing the unexpected database state or causing unexpected actions to result from leaving the database in an inconsistent state).

## Scanner
Discovers photos in a given path, inserting them to the database in batches of a configurable amount.
It should also insert various metadata into the records in the DB when reading the image.
It should run in multiple threads to saturate file system IO.

## Hasher
Reads all images that don't have hashes in the DB.
The main thread:
- Starts a configurable number of worker threads.
- Generates batches of images from the database and feeds those to the workers, keeping a fixed size queue relatively full.
- Displays a progress bar showing how many images it will hash vs how many are in progress / submitted vs how many have been completed.
- Displays a speed indicator.
- Receives the hash result batches from worker threads and updates / inserts the results into the database in configurable sized batches.

The worker threads:
- Receive batches of images to hash.
- Perform the file contents hash with a configurable algorithm.
- Perform the perceptual hash using a configurable algorithm.
- Send the processed batch and hash details back to the main thread.

## Grouper
A separate process like the hasher or scanner.
Assigns images with the same or similar hashes in the DB to the same groups.
- perfect matches
- perceptual hash >= config value (99% similar to start).
There should be a group for each set of perfect matches with the same hash and a group for each set of perceptually similar images.
Running this process re-groups all images.

## Testing
There will be automated unit test cases and integration test cases for all functionality.
Each piece of simple functionality needs at least a positive and a negative test case (like scanner finds these files, scanner doesn't find these files).
The web UI tests can be done using an automated browser framework for Chrome.
All test case orchestration should use a standard Rust testing library.


