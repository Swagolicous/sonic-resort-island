// Recovers MSVC RTTI on the Xbox 360 image and names virtual methods.
//
//   TypeDescriptor          { u32 vftable; u32 spare; char name[] = ".?AV...@@" }
//   CompleteObjectLocator   { u32 signature=0; u32 offset; u32 cdOffset; u32 typeDescriptor; u32 hierarchy }
//   vtable                  [-1] -> CompleteObjectLocator, [0..] -> methods
//
// Each class gets a Ghidra namespace (e.g. Sonic::Player::CPlayerSpeed) and a
// labelled vtable. Unnamed functions referenced from vtables are moved into the
// namespace of the class with the *shortest* vtable that contains them (the
// most-base class that introduced the slot) and named vf<slot>.
// Writes rtti_classes.jsonl (class, type descriptor, vtable, methods) to the
// directory given as the first script argument.
//@category UnleashedRE
import ghidra.app.script.GhidraScript;
import ghidra.app.util.NamespaceUtils;
import ghidra.program.model.address.Address;
import ghidra.program.model.address.AddressSpace;
import ghidra.program.model.listing.Function;
import ghidra.program.model.mem.Memory;
import ghidra.program.model.mem.MemoryBlock;
import ghidra.program.model.symbol.Namespace;
import ghidra.program.model.symbol.SourceType;

import java.io.File;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.*;

public class NameRttiVtables extends GhidraScript {
    private Memory mem;
    private AddressSpace space;

    @Override
    public void run() throws Exception {
        String[] args = getScriptArgs();
        File outDir = new File(args.length > 0 ? args[0] : ".");
        outDir.mkdirs();
        mem = currentProgram.getMemory();
        space = currentProgram.getAddressFactory().getDefaultAddressSpace();

        // 1. Type descriptors.
        Map<Long, String> types = new HashMap<>();
        for (MemoryBlock block : mem.getBlocks()) {
            if (!block.isInitialized() || block.isExecute()) continue;
            byte[] bytes = new byte[(int) block.getSize()];
            block.getBytes(block.getStart(), bytes);
            long base = block.getStart().getOffset();
            for (int i = 8; i + 4 < bytes.length; i++) {
                if (bytes[i] == '.' && bytes[i + 1] == '?' && bytes[i + 2] == 'A'
                        && (bytes[i + 3] == 'V' || bytes[i + 3] == 'U')) {
                    int end = i;
                    while (end < bytes.length && bytes[end] != 0) end++;
                    types.put(base + i - 8, new String(bytes, i, end - i, StandardCharsets.US_ASCII));
                }
            }
        }
        println("RTTI: " + types.size() + " type descriptors");

        // 2. Complete object locators and 3. vtables pointing at them.
        Map<Long, Long> colToType = new HashMap<>();
        Map<Long, Integer> colOffset = new HashMap<>();
        for (MemoryBlock block : mem.getBlocks()) {
            if (!block.isInitialized() || block.isExecute()) continue;
            byte[] b = new byte[(int) block.getSize()];
            block.getBytes(block.getStart(), b);
            long base = block.getStart().getOffset();
            for (int i = 0; i + 20 <= b.length; i += 4) {
                if (be32(b, i) == 0) {
                    long td = be32(b, i + 12);
                    if (types.containsKey(td)) {
                        colToType.put(base + i, td);
                        colOffset.put(base + i, (int) be32(b, i + 4));
                    }
                }
            }
        }
        println("RTTI: " + colToType.size() + " complete object locators");

        Map<Long, Long> vtableToCol = new LinkedHashMap<>();
        for (MemoryBlock block : mem.getBlocks()) {
            if (!block.isInitialized() || block.isExecute()) continue;
            byte[] b = new byte[(int) block.getSize()];
            block.getBytes(block.getStart(), b);
            long base = block.getStart().getOffset();
            for (int i = 0; i + 8 <= b.length; i += 4) {
                long v = be32(b, i);
                if (colToType.containsKey(v)) {
                    vtableToCol.put(base + i + 4, v);
                }
            }
        }
        println("RTTI: " + vtableToCol.size() + " vtables");

        // Collect methods per vtable.
        Map<Long, List<Long>> methods = new LinkedHashMap<>();
        Set<Long> vtableStarts = vtableToCol.keySet();
        for (long vt : vtableStarts) {
            List<Long> slots = new ArrayList<>();
            for (int k = 0; k < 4096; k++) {
                long at = vt + 4L * k;
                if (k > 0 && vtableStarts.contains(at + 4)) break; // next vtable's COL slot
                Address a = space.getAddress(at);
                if (!mem.contains(a)) break;
                long target = mem.getInt(a) & 0xffffffffL;
                Address t;
                try {
                    t = space.getAddress(target);
                } catch (Exception e) {
                    break;
                }
                MemoryBlock tb = mem.getBlock(t);
                if (tb == null || !tb.isExecute() || (target & 3) != 0) break;
                slots.add(target);
            }
            methods.put(vt, slots);
        }

        // Assign each function to the shortest vtable containing it.
        Map<Long, long[]> owner = new HashMap<>(); // fn -> {vtable, slot, length}
        for (Map.Entry<Long, List<Long>> e : methods.entrySet()) {
            List<Long> slots = e.getValue();
            for (int k = 0; k < slots.size(); k++) {
                long fn = slots.get(k);
                long[] cur = owner.get(fn);
                if (cur == null || slots.size() < cur[2]) {
                    owner.put(fn, new long[] {e.getKey(), k, slots.size()});
                }
            }
        }

        int renamed = 0;
        int created = 0;
        Map<Long, Namespace> nsCache = new HashMap<>();
        try (PrintWriter out = new PrintWriter(new File(outDir, "rtti_classes.jsonl"), "UTF-8")) {
            for (Map.Entry<Long, Long> e : vtableToCol.entrySet()) {
                long vt = e.getKey();
                long col = e.getValue();
                long td = colToType.get(col);
                String mangled = types.get(td);
                String cls = demangle(mangled);
                Namespace ns = nsCache.get(td);
                if (ns == null) {
                    try {
                        ns = NamespaceUtils.createNamespaceHierarchy(cls, null, currentProgram, SourceType.ANALYSIS);
                    } catch (Exception ex) {
                        ns = null;
                    }
                    nsCache.put(td, ns);
                }
                int off = colOffset.getOrDefault(col, 0);
                String label = off == 0 ? "vftable" : "vftable_" + Integer.toHexString(off);
                try {
                    createLabel(space.getAddress(vt), label, ns, false, SourceType.ANALYSIS);
                } catch (Exception ex) {
                    // label collisions are harmless
                }
                List<Long> slots = methods.get(vt);
                StringBuilder sb = new StringBuilder();
                sb.append("{\"class\":\"").append(json(cls)).append("\",\"mangled\":\"").append(json(mangled))
                  .append("\",\"type_descriptor\":\"").append(hex(td)).append("\",\"vtable\":\"").append(hex(vt))
                  .append("\",\"offset\":").append(off).append(",\"methods\":[");
                for (int k = 0; k < slots.size(); k++) {
                    long fnAddr = slots.get(k);
                    if (k > 0) sb.append(',');
                    sb.append('"').append(hex(fnAddr)).append('"');
                    long[] own = owner.get(fnAddr);
                    if (own == null || own[0] != vt || ns == null) continue;
                    Address fa = space.getAddress(fnAddr);
                    Function f = getFunctionAt(fa);
                    if (f == null) {
                        f = createFunction(fa, null);
                        if (f != null) created++;
                    }
                    if (f != null && (f.getSymbol().getSource() == SourceType.DEFAULT || f.getName().startsWith("FUN_"))) {
                        try {
                            f.setParentNamespace(ns);
                            f.setName("vf" + k, SourceType.ANALYSIS);
                            renamed++;
                        } catch (Exception ex) {
                            // duplicate name in namespace; keep default
                        }
                    }
                }
                sb.append("]}");
                out.println(sb);
            }
        }
        println("RTTI: named " + renamed + " virtual methods (" + created + " functions created)");
    }

    private static long be32(byte[] b, int i) {
        return ((b[i] & 0xffL) << 24) | ((b[i + 1] & 0xffL) << 16) | ((b[i + 2] & 0xffL) << 8) | (b[i + 3] & 0xffL);
    }

    private static String hex(long v) {
        return String.format("%08X", v);
    }

    private static String json(String s) {
        return s.replace("\\", "\\\\").replace("\"", "\\\"");
    }

    /** ".?AVCPlayerSpeed@Player@Sonic@@" -> "Sonic::Player::CPlayerSpeed". */
    static String demangle(String m) {
        String s = m.substring(4);
        if (s.endsWith("@@")) s = s.substring(0, s.length() - 2);
        if (s.contains("?") || s.contains("$")) {
            return "Templates::" + s.replaceAll("[^A-Za-z0-9_]", "_");
        }
        String[] parts = s.split("@");
        StringBuilder sb = new StringBuilder();
        for (int i = parts.length - 1; i >= 0; i--) {
            if (parts[i].isEmpty()) continue;
            if (sb.length() > 0) sb.append("::");
            sb.append(parts[i]);
        }
        return sb.toString();
    }
}
