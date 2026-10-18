//
//  ░█▀█░█▀█░█▀█░█▀▀░█░░░▀█▀░█▀▀░▀█▀
//  ░█▀▀░█▀█░█░█░█▀▀░█░░░░█░░▀▀█░░█░
//  ░▀░░░▀░▀░▀░▀░▀▀▀░▀▀▀░▀▀▀░▀▀▀░░▀░
//
//  Panelist — Strongly Typed Grafana Dashboards
//  https://github.com/prisma-risk/panelist
//
//  Copyright (c) 2026 Prisma Risk
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//      https://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
//

use panelist::prelude::*;

fn main() {
    let _ = dashboard! {
        title: "Nested";
        table "Routes" {
            query: promql!("up") { formt: table; }
            query: loki!("{job=\"api\"}") { format: table; }
            override field("p95") { colour: red; }
            override fiield("p99") { unit: seconds; }
            transform organise { hide "Time"; }
            transform sort_by("p95", descending);
            cell: gauge { mod: lcd; };
        }
        variable "instance" {
            refresh: onload;
            sortt: numerical_asc;
        }
    };
}
