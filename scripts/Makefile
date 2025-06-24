.PHONY: dev build clean

dev:
	tmuxp load solana_localnet.yml

build:
	cargo build-bpf

clean:
	cargo clean
	rm -f alice.json bob.json mint.addr alice_ata.addr bob_ata.addr
