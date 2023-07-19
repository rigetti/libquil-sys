FROM ubuntu:22.04

WORKDIR /usr/src

RUN apt update && apt install -y sbcl git build-essential
RUN git clone --branch sbcl-2.2.0 git://git.code.sf.net/p/sbcl/sbcl
RUN cd sbcl && sh make.sh && sh make-shared-library.sh
RUN apt remove -y sbcl
RUN cd sbcl && sh install.sh && cp src/runtime/libsbcl.so /usr/local/lib/libsbcl.so

ARG QUICKLISP_VERSION=2022-04-01
ARG QUICKLISP_URL=http://beta.quicklisp.org/dist/quicklisp/${QUICKLISP_VERSION}/distinfo.txt

WORKDIR /usr/src/lisp
RUN apt install -y wget
RUN wget -P /tmp/ 'https://beta.quicklisp.org/quicklisp.lisp' \
    && sbcl --noinform --non-interactive --load /tmp/quicklisp.lisp \
            --eval "(quicklisp-quickstart:install :dist-url \"${QUICKLISP_URL}\")" \
    && sbcl --noinform --non-interactive --load ~/quicklisp/setup.lisp \
            --eval '(ql-util:without-prompting (ql:add-to-init-file))' \
    && echo '#+quicklisp(push (truename "/usr/src/lisp") ql:*local-project-directories*)' >> ~/.sbclrc \
    && rm -f /tmp/quicklisp.lisp


RUN git clone https://github.com/notmgsk/quilc.git && cd quilc && git checkout f60b035b4d5e2fbf82562e57500a4ea0ed927547 
RUN git clone https://github.com/quil-lang/qvm.git && cd qvm && git checkout 4617625cb6053b1adfd3f7aea9cd2be328b225f6
RUN git clone https://github.com/quil-lang/magicl.git && cd magicl && git checkout c2a9d9ecd859db35d44286af310b7ab99f5f9d77
RUN git clone https://github.com/quil-lang/sbcl-librarian.git && cd sbcl-librarian && git checkout 8100eaecb32cd438c19886c9026088f39cdfa81b

RUN apt install -y libblas-dev libffi-dev libffi7 liblapack-dev libz-dev libzmq3-dev gfortran libssl-dev
RUN make -C quilc
RUN make -C quilc/lib