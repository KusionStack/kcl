<h1 align="center">KCL: Constraint-based Record & Functional Language</h1>

<p align="center">
<a href="./README.md">English</a> | <a href="./README-zh.md">简体中文</a>
</p>
<p align="center">
<a href="#introduction">Introduction</a> | <a href="#features">Features</a> | <a href="#what-is-it-for">What is it for</a> | <a href="#installation">Installation</a> | <a href="#showcase">Showcase</a> | <a href="#documentation">Documentation</a> | <a href="#contributing">Contributing</a> | <a href="#roadmap">Roadmap</a>
</p>

<p align="center">
  <img src="https://github.com/KusionStack/KCLVM/workflows/KCL/badge.svg">
  <img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg?style=flat-square">
  <img src="https://coveralls.io/repos/github/KusionStack/KCLVM/badge.svg">
  <img src="https://img.shields.io/github/release/KusionStack/KCLVM.svg">
  <img src="https://img.shields.io/github/license/KusionStack/KCLVM.svg">
</p>

## Introduction

Kusion Configuration Language (KCL) is an open source constraint-based record and functional language. KCL improves the writing of a large number of complex configurations such as cloud native scenarios through mature programming language technology and practice, and is committed to building better modularity, scalability and stability around configuration, simpler logic writing, fast automation and good ecological extensionality.

## What is it for?

You can use KCL to

+ [Generate low-level static configuration data](https://kcl-lang.github.io/docs/user_docs/guides/configuration) like JSON, YAML, etc or [integrate with existing data](https://kcl-lang.github.io/docs/user_docs/guides/data-integration).
+ Reduce boilerplate in configuration data with the [schema modeling](https://kcl-lang.github.io/docs/user_docs/guides/schema-definition).
+ Define schemas with [rule constraints for configuration data and validate](https://kcl-lang.github.io/docs/user_docs/guides/validation) them automatically].
+ Organize, simplify, unify and manage large configurations without side effects through [gradient automation schemes](https://kcl-lang.github.io/docs/user_docs/guides/automation).
+ Manage large configurations scalably with [isolated configuration blocks](https://kcl-lang.github.io/docs/reference/lang/tour#config-operations).
+ Used as a platform engineering programming language to deliver modern app with [Kusion Stack](https://kusionstack.io).

## Features

+ **Easy-to-use**: Originated from high-level languages ​​such as Python and Golang, incorporating functional language features with low side effects.
+ **Well-designed**: Independent Spec-driven syntax, semantics, runtime and system modules design.
+ **Quick modeling**: [Schema](https://kcl-lang.github.io/docs/reference/lang/tour#schema)-centric configuration types and modular abstraction.
+ **Rich capabilities**: Configuration with type, logic and policy based on [Config](https://kcl-lang.github.io/docs/reference/lang/tour#config-operations), [Schema](https://kcl-lang.github.io/docs/reference/lang/tour#schema), [Lambda](https://kcl-lang.github.io/docs/reference/lang/tour#function), [Rule](https://kcl-lang.github.io/docs/reference/lang/tour#rule).
+ **Stability**: Configuration stability built on [static type system](https://kcl-lang.github.io/docs/reference/lang/tour/#type-system), [constraints](https://kcl-lang.github.io/docs/reference/lang/tour/#validation), and [rules](https://kcl-lang.github.io/docs/reference/lang/tour#rule).
+ **Scalability**: High scalability through [automatic merge mechanism](https://kcl-lang.github.io/docs/reference/lang/tour/#-operators-1) of isolated config blocks.
+ **Fast automation**: Gradient automation scheme of [CRUD APIs](https://kcl-lang.github.io/docs/reference/lang/tour/#kcl-cli-variable-override), [multilingual SDKs](https://kcl-lang.github.io/docs/reference/xlang-api/overview), [language plugin](https://github.com/KusionStack/kcl-plugin)
+ **High performance**: High compile time and runtime performance using Rust & C and [LLVM](https://llvm.org/), and support compilation to native code and [WASM](https://webassembly.org/).
+ **API affinity**: Native support API ecological specifications such as [OpenAPI](https://github.com/KusionStack/kcl-openapi), Kubernetes CRD, Kubernetes YAML spec.
+ **Development friendly**: Friendly development experiences with rich [language tools](https://kcl-lang.github.io/docs/tools/cli/kcl/) (Format, Lint, Test, Vet, Doc, etc.) and [IDE plugins](https://github.com/KusionStack/vscode-kcl).
+ **Safety & maintainable**: Domain-oriented, no system-level functions such as native threads and IO, low noise and security risk, easy maintenance and governance.
+ **Production-ready**: Widely used in production practice of platform engineering and automation at Ant Group.

## How to choose?

The simple answer:

+ YAML is recommended if you need to write structured static K-V, or use Kubernetes' native tools
+ HCL is recommended if you want to use programming language convenience to remove boilerplate with good human readability, or if you are already a Terraform user
+ CUE is recommended if you want to use type system to improve stability and maintain scalable configurations
+ KCL is recommended if you want types and modeling like a modern language, scalable configurations, in-house pure functions and rules, and production-ready performance and automation

A detailed feature and scenario comparison is [here](https://kcl-lang.github.io/docs/user_docs/getting-started/intro).

## Installation

[Download](https://github.com/KusionStack/KCLVM/releases) the latest release from GitHub and add `{install-location}/kclvm/bin` to the environment `PATH`.

## Showcase

`./samples/kubernetes.k` is an example of generating kubernetes manifests.

```python
apiVersion = "apps/v1"
kind = "Deployment"
metadata = {
    name = "nginx"
    labels.app = "nginx"
}
spec = {
    replicas = 3
    selector.matchLabels = metadata.labels
    template.metadata.labels = metadata.labels
    template.spec.containers = [
        {
            name = metadata.name
            image = "${metadata.name}:1.14.2"
            ports = [{ containerPort = 80 }]
        }
    ]
}
```

We can execute the following command to get a YAML output.

```bash
kcl ./samples/kubernetes.k
```

YAML output

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: nginx
  labels:
    app: nginx
spec:
  replicas: 3
  selector:
    matchLabels:
      app: nginx
  template:
    metadata:
      labels:
        app: nginx
    spec:
      containers:
      - name: nginx
        image: nginx:1.14.2
        ports:
        - containerPort: 80
```

## Documentation

Detailed documentation is available at [KCL Website](https://kcl-lang.github.io/)

## Contributing

See [Developing Guide](./docs/dev_guide/1.about_this_guide.md).

## Roadmap

See [KCL Roadmap](https://github.com/KusionStack/KCLVM/issues/29).

## Community

See the [community](https://github.com/KusionStack/community) for ways to join us.
