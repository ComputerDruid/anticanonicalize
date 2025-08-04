anticanonicalize is a tool which runs a command with the current working directory set to an "unreachable" version of the directory.

Some things that work:
- `anticanonicalize ls`
- `anticanonicalize bash -i` (although it'll print some warnings and show "." as the cwd on the prompt)

Some things that don't work:
- `anticanonicalize pwd` (this is the whole point)
- `anticanonicalize cargo check` (`error: Unable to proceed. Could not locate working directory.: No such file or directory (os error 2)
`)

## How?

Basically:
- `unshare --user --mount --map-root` to make a mount namespace, and inside:
  - `mount --bind . $some_tmpdir` to bind mount it somewhere that won't be visible outside the namespace
  - open a file descriptor for `$some_tmpdir` and send it back outside the mount namespace with a unix domain socket.
- `fchdir` the received file descriptor to cd there
- `exec` the specified command

## Why??

Well, it's fun!

Also, this is theoretically useful as a tool to prove that some command does not rely on the absolute paths of its inputs. In general, I want that property in build systems because I want to be able to share build caches between multiple checkouts/worktrees.

Unfortunately, in practice this breaks many many things, so it's unlikely you can use this technique to prove anything about real builds.

Also, a given tool could have fallback logic to handle this case, so unless you always run your build with anticanonicalize, it still might rely on absolute paths when you don't run it that way.

## License

Copyright 2025 Daniel Johnson et al.

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE2](LICENSE-APACHE2) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
