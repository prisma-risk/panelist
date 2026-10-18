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
        title: "Features";
        variable "instance" { values: ["a"]; }
        stat "Mappings" {
            mapping range(0.0) => "Low";
            mapping special(nul) => "Missing";
            repeat: horizontal();
            color: continuous(blue_yelow_red);
            unit: currency(euro);
            data_link "Logs" => "/explore" { tooltip: "Logs"; }
        }
        annotation "Restarts" => "changes(process_start_time_seconds[2m]) > 0" {
            icon: red;
        }
    };
}
