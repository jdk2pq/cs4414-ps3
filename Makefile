all: zhtta

zhtta: zhtta.rs
	rustc zhtta.rs

clean: 
	rm -rf *~ *.rs.orig
