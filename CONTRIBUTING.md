## Workflow
- Major releases are determined by batches of new features, and are versioned by X.Y++.Z
  - For example, release 0.3.0 is triggered by completion of the new map feature (plus others)
- Minor releases are determined by bug fixes and minor UX updates, and are versioned by X.Y.Z++
  - For example, release 0.2.1 is triggered by a bug fix for 0.2.0
- Release branches are created once a certain set of new features are implemented. It is then tested
  - No new features are added, and bug fixes are cherry picked during the stabilization process
  - Each set of bug fixes triggers a minor release 
 
## Making a bug report
1. Create the Issue here: https://github.com/yoshidonoshi/stork-editor/issues/new
2. Give it a descriptive name and apply the "bug" label
3. In the body, add the following:
- Discovered Location (world, level, map, background)
- What happened (descriptive step by step)
- Screenshots (if possible, one or more screenshots elaborating on the issue)
- Log (attach your `stork.log` file)
4. A good example issue: https://github.com/yoshidonoshi/stork-editor/issues/4

## Fixing a bug
1. If you haven't already, fork the repository
2. If there isn't already a branch for it, create a branch called "bug/problem-name" (example: bug/tileset-loading-wrong)
3. Make your fix commit. Do not include anything unrelated to fixing the bug
4. Create a pull request to Main, or if the bug already has a branch on the primary repository, make the PR to that branch
5. Wait for approval or change requests

## Suggesting features
- The most valuable feature suggestions are UX related, as in what to do to make the editor more comfortable and easy to use
- Ensure the feature isn't already proposed or implemented in the repo
- Create the issue and apply the "enhancement" label

## Contributing features
0. **There must be a new feature issue created beforehand to ensure compatibility with the project**
1. If you haven't already, fork the repository
2. If there isn't already a branch, create one and name it "feature/feature-name"
3. Make a PR to the Main branch
4. Wait for approval or change requests
