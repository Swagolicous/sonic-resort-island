// Stream Ghidra analysis into small, address-joinable JSONL relations.
//
// Args:
//   <output-dir>
//
// Writes:
//   addresses.jsonl
//   names.jsonl
//   functions.jsonl
//   strings.jsonl
//   call_edges.jsonl
//
// All records include binary/program provenance.  Direct call edges are only
// Ghidra reference-manager calls; they are not a complete model of indirect
// dispatch, virtual calls, or computed branches.
//
// @category UnleashedRecomp

import ghidra.app.script.GhidraScript;
import ghidra.framework.Application;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressIterator;
import ghidra.program.model.address.AddressRange;
import ghidra.program.model.address.AddressRangeIterator;
import ghidra.program.model.address.AddressSetView;
import ghidra.program.model.listing.Data;
import ghidra.program.model.listing.DataIterator;
import ghidra.program.model.listing.Function;
import ghidra.program.model.listing.FunctionIterator;
import ghidra.program.model.listing.FunctionManager;
import ghidra.program.model.listing.Listing;
import ghidra.program.model.symbol.Reference;
import ghidra.program.model.symbol.ReferenceManager;
import ghidra.program.model.symbol.SourceType;
import ghidra.program.model.symbol.Symbol;
import ghidra.program.model.symbol.SymbolIterator;
import ghidra.program.model.symbol.SymbolTable;

import java.io.BufferedWriter;
import java.io.File;
import java.io.FileOutputStream;
import java.io.OutputStreamWriter;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.Set;

public class ExportJoinJsonl extends GhidraScript {
    private String programName;
    private String binarySha256;
    private String executableFormat;
    private String ghidraVersion;

    private static String json(String s) {
        if (s == null) return "null";
        StringBuilder b = new StringBuilder(s.length() + 16);
        b.append('"');
        for (int i = 0; i < s.length(); i++) {
            char c = s.charAt(i);
            switch (c) {
                case '"': b.append("\\\""); break;
                case '\\': b.append("\\\\"); break;
                case '\b': b.append("\\b"); break;
                case '\f': b.append("\\f"); break;
                case '\n': b.append("\\n"); break;
                case '\r': b.append("\\r"); break;
                case '\t': b.append("\\t"); break;
                default:
                    if (c < 0x20) {
                        b.append(String.format("\\u%04x", (int)c));
                    } else {
                        b.append(c);
                    }
            }
        }
        b.append('"');
        return b.toString();
    }

    private static String hex(Address a) {
        if (a == null) return null;
        return String.format("0x%X", a.getUnsignedOffset());
    }

    private static String u64(Address a) {
        if (a == null) return "null";
        return Long.toUnsignedString(a.getUnsignedOffset());
    }

    private String common(String schema, String recordType) {
        return "\"schema\":" + json(schema) +
            ",\"record_type\":" + json(recordType) +
            ",\"program\":" + json(programName) +
            ",\"binary_sha256\":" + json(binarySha256) +
            ",\"executable_format\":" + json(executableFormat) +
            ",\"provenance\":{\"producer\":\"ExportJoinJsonl.java\"" +
            ",\"ghidra_version\":" + json(ghidraVersion) + "}";
    }

    private static PrintWriter open(File f) throws Exception {
        return new PrintWriter(new BufferedWriter(new OutputStreamWriter(
            new FileOutputStream(f), StandardCharsets.UTF_8), 1024 * 1024));
    }

    private static long bodySize(AddressSetView body) {
        long n = 0;
        AddressRangeIterator ranges = body.getAddressRanges();
        while (ranges.hasNext()) {
            AddressRange r = ranges.next();
            n += r.getLength();
        }
        return n;
    }

    private static String sourceName(SourceType source) {
        return source == null ? null : source.toString();
    }

    private static String functionConfidence(Function f) {
        SourceType s = f.getSymbol().getSource();
        if (s == SourceType.USER_DEFINED) return "analyst";
        if (s == SourceType.IMPORTED) return "imported";
        if (s == SourceType.ANALYSIS) return "analysis";
        return "default";
    }

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        if (args.length != 1) {
            throw new IllegalArgumentException(
                "usage: ExportJoinJsonl.java <output-dir>");
        }

        File outDir = new File(args[0]).getAbsoluteFile();
        if (!outDir.exists() && !outDir.mkdirs()) {
            throw new IllegalArgumentException("cannot create output dir: " + outDir);
        }
        if (!outDir.isDirectory()) {
            throw new IllegalArgumentException("not a directory: " + outDir);
        }

        programName = currentProgram.getName();
        binarySha256 = currentProgram.getExecutableSHA256();
        if (binarySha256 != null) binarySha256 = binarySha256.toLowerCase();
        executableFormat = currentProgram.getExecutableFormat();
        ghidraVersion = Application.getApplicationVersion();

        File addressesFile = new File(outDir, "addresses.jsonl");
        File namesFile = new File(outDir, "names.jsonl");
        File functionsFile = new File(outDir, "functions.jsonl");
        File stringsFile = new File(outDir, "strings.jsonl");
        File callsFile = new File(outDir, "call_edges.jsonl");

        long addressRows = 0;
        long nameRows = 0;
        long functionRows = 0;
        long stringRows = 0;
        long callRows = 0;

        FunctionManager fm = currentProgram.getFunctionManager();
        SymbolTable st = currentProgram.getSymbolTable();
        Listing listing = currentProgram.getListing();
        ReferenceManager refs = currentProgram.getReferenceManager();

        try (
            PrintWriter addressOut = open(addressesFile);
            PrintWriter nameOut = open(namesFile);
            PrintWriter functionOut = open(functionsFile);
            PrintWriter stringOut = open(stringsFile);
            PrintWriter callOut = open(callsFile)
        ) {
            // Address relation: one row for every function entry, defined string,
            // and non-external symbol address.  Duplicates by role are intentional;
            // (binary_sha256,address_u64,role) is the natural join key.
            FunctionIterator fi = fm.getFunctions(true);
            while (fi.hasNext()) {
                monitor.checkCancelled();
                Function f = fi.next();
                Address a = f.getEntryPoint();
                addressOut.println("{" +
                    common("unleashed_re.ghidra.address.v1", "address") +
                    ",\"address_hex\":" + json(hex(a)) +
                    ",\"address_u64\":" + u64(a) +
                    ",\"address_space\":" + json(a.getAddressSpace().getName()) +
                    ",\"role\":\"function_entry\"" +
                    ",\"confidence\":\"ghidra_function\"" +
                    "}");
                addressRows++;
            }

            DataIterator di = listing.getDefinedData(true);
            while (di.hasNext()) {
                monitor.checkCancelled();
                Data d = di.next();
                if (!d.hasStringValue()) continue;
                Address a = d.getAddress();

                String value;
                try {
                    Object v = d.getValue();
                    value = v == null ? null : v.toString();
                } catch (Exception e) {
                    value = null;
                }

                stringOut.println("{" +
                    common("unleashed_re.ghidra.string.v1", "string") +
                    ",\"address_hex\":" + json(hex(a)) +
                    ",\"address_u64\":" + u64(a) +
                    ",\"address_space\":" + json(a.getAddressSpace().getName()) +
                    ",\"value\":" + json(value) +
                    ",\"byte_length\":" + d.getLength() +
                    ",\"data_type\":" + json(d.getDataType().getName()) +
                    ",\"confidence\":\"defined_data\"" +
                    "}");
                stringRows++;

                addressOut.println("{" +
                    common("unleashed_re.ghidra.address.v1", "address") +
                    ",\"address_hex\":" + json(hex(a)) +
                    ",\"address_u64\":" + u64(a) +
                    ",\"address_space\":" + json(a.getAddressSpace().getName()) +
                    ",\"role\":\"defined_string\"" +
                    ",\"confidence\":\"defined_data\"" +
                    "}");
                addressRows++;
            }

            SymbolIterator si = st.getAllSymbols(true);
            while (si.hasNext()) {
                monitor.checkCancelled();
                Symbol s = si.next();
                Address a = s.getAddress();
                if (a == null || a.isExternalAddress()) continue;

                nameOut.println("{" +
                    common("unleashed_re.ghidra.name.v1", "name") +
                    ",\"address_hex\":" + json(hex(a)) +
                    ",\"address_u64\":" + u64(a) +
                    ",\"address_space\":" + json(a.getAddressSpace().getName()) +
                    ",\"name\":" + json(s.getName()) +
                    ",\"namespace\":" + json(s.getParentNamespace().getName(true)) +
                    ",\"symbol_type\":" + json(s.getSymbolType().toString()) +
                    ",\"source_type\":" + json(sourceName(s.getSource())) +
                    ",\"primary\":" + s.isPrimary() +
                    ",\"confidence\":" + json(
                        s.getSource() == SourceType.USER_DEFINED ? "analyst" :
                        s.getSource() == SourceType.IMPORTED ? "imported" :
                        s.getSource() == SourceType.ANALYSIS ? "analysis" : "default") +
                    "}");
                nameRows++;

                addressOut.println("{" +
                    common("unleashed_re.ghidra.address.v1", "address") +
                    ",\"address_hex\":" + json(hex(a)) +
                    ",\"address_u64\":" + u64(a) +
                    ",\"address_space\":" + json(a.getAddressSpace().getName()) +
                    ",\"role\":\"symbol\"" +
                    ",\"confidence\":" + json(
                        s.getSource() == SourceType.USER_DEFINED ? "analyst" :
                        s.getSource() == SourceType.IMPORTED ? "imported" :
                        s.getSource() == SourceType.ANALYSIS ? "analysis" : "default") +
                    "}");
                addressRows++;
            }

            fi = fm.getFunctions(true);
            while (fi.hasNext()) {
                monitor.checkCancelled();
                Function f = fi.next();
                Address entry = f.getEntryPoint();
                AddressSetView body = f.getBody();
                Address bodyMin = body.getMinAddress();
                Address bodyMax = body.getMaxAddress();

                String signature;
                try {
                    signature = f.getPrototypeString(false, false);
                } catch (Exception e) {
                    signature = null;
                }

                functionOut.println("{" +
                    common("unleashed_re.ghidra.function.v1", "function") +
                    ",\"address_hex\":" + json(hex(entry)) +
                    ",\"address_u64\":" + u64(entry) +
                    ",\"address_space\":" + json(entry.getAddressSpace().getName()) +
                    ",\"name\":" + json(f.getName()) +
                    ",\"name_source\":" + json(sourceName(f.getSymbol().getSource())) +
                    ",\"body_min_hex\":" + json(hex(bodyMin)) +
                    ",\"body_max_hex\":" + json(hex(bodyMax)) +
                    ",\"body_size\":" + bodySize(body) +
                    ",\"signature\":" + json(signature) +
                    ",\"calling_convention\":" + json(f.getCallingConventionName()) +
                    ",\"is_thunk\":" + f.isThunk() +
                    ",\"is_external\":" + f.isExternal() +
                    ",\"is_varargs\":" + f.hasVarArgs() +
                    ",\"confidence\":" + json(functionConfidence(f)) +
                    "}");
                functionRows++;

                AddressIterator srcIt = refs.getReferenceSourceIterator(body, true);
                Set<String> emittedAtFunction = new HashSet<String>();
                while (srcIt.hasNext()) {
                    Address from = srcIt.next();
                    for (Reference ref : refs.getReferencesFrom(from)) {
                        if (!ref.getReferenceType().isCall()) continue;

                        Address to = ref.getToAddress();
                        Function callee = fm.getFunctionAt(to);
                        String calleeName = callee == null ? null : callee.getName();
                        String edgeKey = from.toString() + "|" + to.toString() + "|" +
                            ref.getReferenceType().toString();

                        // One reference per call site/type/target.  The set prevents
                        // duplicate references exposed by overlapping source iterators.
                        if (!emittedAtFunction.add(edgeKey)) continue;

                        callOut.println("{" +
                            common("unleashed_re.ghidra.call_edge.v1", "call_edge") +
                            ",\"caller_address_hex\":" + json(hex(entry)) +
                            ",\"caller_address_u64\":" + u64(entry) +
                            ",\"caller_name\":" + json(f.getName()) +
                            ",\"callsite_address_hex\":" + json(hex(from)) +
                            ",\"callsite_address_u64\":" + u64(from) +
                            ",\"target_address_hex\":" + json(hex(to)) +
                            ",\"target_address_u64\":" + u64(to) +
                            ",\"callee_name\":" + json(calleeName) +
                            ",\"reference_type\":" + json(ref.getReferenceType().toString()) +
                            ",\"target_is_function_entry\":" + (callee != null) +
                            ",\"confidence\":\"direct_reference\"" +
                            ",\"completeness\":\"direct_calls_only\"" +
                            "}");
                        callRows++;
                    }
                }
            }
        }

        println("output_dir=" + outDir.getAbsolutePath());
        println("addresses=" + addressRows +
            " names=" + nameRows +
            " functions=" + functionRows +
            " strings=" + stringRows +
            " call_edges=" + callRows);
        println("binary_sha256=" + binarySha256);
    }
}
