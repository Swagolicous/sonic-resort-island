// Decompiles every non-thunk function in parallel and writes C source to a
// clean output tree, plus a JSONL manifest used by the gamedb parity check.
//
// Layout (first script argument = output root):
//   src/<Namespace>/<Class>.c         functions in an RTTI-recovered class namespace
//   src/_global/<addr & ~0xFFFF>.c    everything else, grouped by 64 KiB of address space
//   manifest.jsonl                    one record per function (address, name, file, lines, sha256, status, callees)
//   failures.txt                      functions the decompiler could not produce
//
// Every function body is preceded by a "// @function <addr> <size> <name>" marker
// so the parity check can round-trip each record back to its source slice.
//@category UnleashedRE
import ghidra.app.decompiler.DecompInterface;
import ghidra.app.decompiler.DecompileOptions;
import ghidra.app.decompiler.DecompileResults;
import ghidra.app.decompiler.parallel.ChunkingParallelDecompiler;
import ghidra.app.decompiler.parallel.DecompilerCallback;
import ghidra.app.decompiler.parallel.ParallelDecompiler;
import ghidra.app.script.GhidraScript;
import ghidra.program.model.listing.Function;
import ghidra.program.model.symbol.Namespace;
import ghidra.util.task.TaskMonitor;

import java.io.File;
import java.io.FileOutputStream;
import java.io.OutputStreamWriter;
import java.io.PrintWriter;
import java.io.Writer;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.*;

public class ExportDecompiledAll extends GhidraScript {
    private static final int CHUNK = 2000;

    static final class Result {
        Function function;
        String code;
        String error;
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        File root = new File(args.length > 0 ? args[0] : "decomp_out");
        File src = new File(root, "src");
        src.mkdirs();

        List<Function> functions = new ArrayList<>();
        for (Function f : currentProgram.getFunctionManager().getFunctions(true)) {
            if (!f.isThunk() && !f.isExternal()) {
                functions.add(f);
            }
        }
        println("ExportDecompiledAll: " + functions.size() + " functions");

        DecompilerCallback<Result> callback = new DecompilerCallback<>(currentProgram, (DecompInterface d) -> {
            DecompileOptions options = new DecompileOptions();
            options.grabFromProgram(currentProgram);
            d.setOptions(options);
            d.toggleCCode(true);
            d.toggleSyntaxTree(false);
            d.setSimplificationStyle("decompile");
        }) {
            @Override
            public Result process(DecompileResults r, TaskMonitor m) {
                Result out = new Result();
                out.function = r.getFunction();
                if (r.decompileCompleted() && r.getDecompiledFunction() != null) {
                    out.code = r.getDecompiledFunction().getC();
                } else {
                    out.error = String.valueOf(r.getErrorMessage()).replace('\n', ' ').trim();
                }
                return out;
            }
        };
        callback.setTimeout(180);
        ChunkingParallelDecompiler<Result> decompiler =
            ParallelDecompiler.createChunkingParallelDecompiler(callback, monitor);

        Map<String, StringBuilder> files = new TreeMap<>();
        Map<String, Integer> lineCounts = new HashMap<>();
        List<String> manifest = new ArrayList<>();
        List<String> failures = new ArrayList<>();
        long started = System.currentTimeMillis();
        int ok = 0;
        for (int i = 0; i < functions.size(); i += CHUNK) {
            monitor.checkCancelled();
            List<Function> chunk = functions.subList(i, Math.min(i + CHUNK, functions.size()));
            List<Result> results = decompiler.decompileFunctions(chunk);
            results.sort(Comparator.comparing(r -> r.function.getEntryPoint()));
            for (Result r : results) {
                Function f = r.function;
                String file = fileFor(f);
                StringBuilder sb = files.computeIfAbsent(file, k -> new StringBuilder("// Decompiled by Ghidra " +
                    getGhidraVersion() + " from " + currentProgram.getName() + "\n\n"));
                int lineStart = lineCounts.getOrDefault(file, 2) + 1;
                String header = String.format("// @function %s %d %s\n", f.getEntryPoint(), f.getBody().getNumAddresses(),
                    f.getName(true));
                String body = r.code != null ? r.code.trim() + "\n" : "// decompilation failed: " + r.error + "\n";
                sb.append(header).append(body).append('\n');
                int lines = countLines(header) + countLines(body) + 1;
                lineCounts.put(file, lineStart - 1 + lines);
                if (r.code != null) {
                    ok++;
                } else {
                    failures.add(f.getEntryPoint() + "\t" + f.getName(true) + "\t" + r.error);
                }
                StringBuilder callees = new StringBuilder();
                for (Function c : f.getCalledFunctions(TaskMonitor.DUMMY)) {
                    if (callees.length() > 0) callees.append(',');
                    callees.append('"').append(c.getEntryPoint()).append('"');
                }
                manifest.add(String.format(
                    "{\"address\":\"%s\",\"name\":\"%s\",\"full_name\":\"%s\",\"size\":%d,\"file\":\"%s\",\"line_start\":%d,\"line_end\":%d,\"status\":\"%s\",\"sha256\":\"%s\",\"callees\":[%s]}",
                    f.getEntryPoint(), json(f.getName()), json(f.getName(true)), f.getBody().getNumAddresses(),
                    json(file), lineStart, lineStart + lines - 2, r.code != null ? "ok" : "failed",
                    sha256(body), callees));
            }
            long secs = (System.currentTimeMillis() - started) / 1000;
            println(String.format("ExportDecompiledAll: %d/%d done (%d ok) after %ds", Math.min(i + CHUNK, functions.size()),
                functions.size(), ok, secs));
        }
        decompiler.dispose();
        callback.dispose();

        List<String> writeErrors = new ArrayList<>();
        for (Map.Entry<String, StringBuilder> e : files.entrySet()) {
            File f = new File(src, e.getKey());
            f.getParentFile().mkdirs();
            try (Writer w = new OutputStreamWriter(new FileOutputStream(f), StandardCharsets.UTF_8)) {
                w.write(e.getValue().toString());
            } catch (Exception ex) {
                writeErrors.add(e.getKey() + "	" + ex.getMessage());
            }
        }
        try (PrintWriter w = new PrintWriter(new File(root, "write_errors.txt"), "UTF-8")) {
            writeErrors.forEach(w::println);
        }
        try (PrintWriter w = new PrintWriter(new File(root, "manifest.jsonl"), "UTF-8")) {
            manifest.forEach(w::println);
        }
        try (PrintWriter w = new PrintWriter(new File(root, "failures.txt"), "UTF-8")) {
            failures.forEach(w::println);
        }
        println(String.format("ExportDecompiledAll: wrote %d files (%d write errors), %d functions (%d ok, %d failed)",
            files.size() - writeErrors.size(), writeErrors.size(), functions.size(), ok, failures.size()));
    }

    private String fileFor(Function f) {
        Namespace ns = f.getParentNamespace();
        if (ns != null && !ns.isGlobal()) {
            // Keep paths well under Windows' MAX_PATH: long template names get
            // a readable prefix plus a stable hash.
            StringBuilder path = new StringBuilder();
            for (String part : ns.getName(true).split("::")) {
                String clean = part.replaceAll("[^A-Za-z0-9_]", "_");
                if (clean.length() > 48) {
                    clean = clean.substring(0, 40) + "_" + String.format("%08x", clean.hashCode());
                }
                if (path.length() > 0) path.append('/');
                path.append(clean);
            }
            if (path.length() > 150) {
                path = new StringBuilder("_long/" + String.format("%08x", path.toString().hashCode()));
            }
            return path + ".c";
        }
        long block = f.getEntryPoint().getOffset() & ~0xFFFFL;
        return String.format("_global/%08X.c", block);
    }

    private static int countLines(String s) {
        int n = 0;
        for (int i = 0; i < s.length(); i++) {
            if (s.charAt(i) == '\n') n++;
        }
        return n;
    }

    private static String json(String s) {
        return s.replace("\\", "\\\\").replace("\"", "\\\"");
    }

    private static String sha256(String s) throws Exception {
        byte[] d = MessageDigest.getInstance("SHA-256").digest(s.getBytes(StandardCharsets.UTF_8));
        StringBuilder sb = new StringBuilder();
        for (byte b : d) sb.append(String.format("%02x", b));
        return sb.toString();
    }
}
