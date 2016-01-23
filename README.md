To-Uni
======

Converts ascii representations of (mostly math-oriented) symbols to the corresponding unicode character. Can make math-heavy LaTeX documents or source code more readable.

Usage
-----
Point the tool at a file and make sure, you have a replacement list lying around somewhere. By default, the tool searches for a file called `to-uni.yml`, starting in the directory of the file you would like to process. If not found, it searches upwards in the file system hierarchy until it finds a config file.

A `to-uni.yml` looks like this:

```yaml
---
patterns:
    alpha: "α"
    beta: "β"
    gamma: "γ"
```

The config file that I commonly use for my work is in [example-files/to-uni.yml](example-files/to-uni.yml). It replaces the letters of the greek alphabet and a number of math symbols supported in common programming fonts.

### In-Place Conversion
```
to-uni my_file.txt
```
By default, this creates a backup in the same directory as `my_file.txt.bak` (replacing any existing file with that name). There is a command line switch to disable this backup. The tool makes some effort not to clobber your input file until it has completed the conversion. For in-place conversions, `to-uni` always first writes to a temporary file and only swaps it with the original file when no errors were detected during the conversion. It uses the [atomicwrites crate](https://crates.io/crates/atomicwrites/) for the final replacement.

### Separate output file
```
to-uni my_file.txt the_output.txt
``` 

I'm not sure if I want to keep this argument form around. It might be more interesting to pass an arbitrary number of files for in-place conversion so that the recognition automaton only has to be computed once for all the files.

## Performance
For my typical use case, performance really didn't matter that much (replace greek characters in <100 page LaTeX document). I still wanted to have good asymptotic behaviour, though, because it sounded like a fun challenge. 

I therefore made sure that the entire conversion happens on a *stream* of bytes, not on entire files. `to-uni` requires a certain amount of space for the recognition automaton, depending on the patterns defined in `to-uni.yml`. See the [memory usage section of the aho-corasick crate](http://burntsushi.net/rustdoc/aho_corasick/#memory-usage) for more details. Because it needs to deal with potentially overlapping patterns, it allocates a sliding window buffer with a size proportional to the length of the longest pattern.

If you have overlapping patterns, some characters might need to be looked at more than once, so the conversion *isn't exactly O(1)*. For example, if you have the patterns `super` and `superpenguin` and the input text contains `superpenga`, then the `peng` part will be scanned at least twice. Once because the system needs to make the distinction between `super` and `superpenguin` and then a second time when `super` has been reported as a match and conversion continues. Technically, this could be avoided by keeping track of matches discovered along the way, but I decided that it wasn't worth the effort.

## License
This tool is licensed under the MIT license. See [LICENSE](./LICENSE) for the full license.

In any event: this piece of software probably has bugs. If it ends up eating the last backup of your files, I'm sorry, but consider yourself warned.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.