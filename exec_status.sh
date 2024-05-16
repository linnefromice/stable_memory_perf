dfx canister update-settings --add-controller $(dfx canister id simple) simple
dfx canister status simple
dfx canister call simple status_all
dfx canister call simple status_used_heap_size
dfx canister call simple status_used_heap_size_utilization
dfx canister call simple status_used_stable_memory
dfx canister call simple status_used_stable_memory_utilization
