# Debugging Nemesis Patches

1. Generate hkx in Nemesis.
2. Convert the required xml in Nemesis/resource to hkx and then to xml again.
   This will generate xml that meets the d-merge specification.
3. Output the xml generated in step 2 to json for 3.
4. Use serde-hkx tool to output Nemesis hkx → json.
5. Diff the results of 3 and 4.
