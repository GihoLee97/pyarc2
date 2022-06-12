#!/usr/bin/env python

import re
import os.path
import requests
import tomli
import sys
import json


def docs_version():

    regexp = re.compile('^release\s?=\s?(.*)$')
    lines = open(os.path.join('docs', 'conf.py')).readlines()

    for line in lines:
        line = line.strip()
        if regexp.match(line):
            m = regexp.match(line)
            return m.group(1).replace("'", "")

    raise ValueError('Could not determine docs version')


def internal_version():

    cargo = tomli.loads(open('Cargo.toml').read())['package']['version']
    pyproject_tool = tomli.loads(open('pyproject.toml').read())['tool']['poetry']['version']
    pyproject = tomli.loads(open('pyproject.toml').read())['project']['version']
    docs = docs_version()

    versions = [cargo, pyproject, pyproject_tool, docs]

    # Check if the same version is used throughout
    consistent = all(v == versions[0] for v in versions)
    if len(versions) < 4:
        raise ValueError('Not all of Cargo.toml, pyproject.toml or docs/conf.py '
            'define versions')
    elif not consistent:
        # complain if it doesn't
        raise ValueError('Cargo.toml, pyproject.toml and docs/conf.py '
            'have inconsistent versions')
    else:
        # return it otherwise
        return versions[0]


def pypi_versions():

    data = requests.get('https://pypi.org/pypi/pyarc2/json')

    if data.status_code != 200:
        raise Exception('Could not determine PyPI version')

    content = json.loads(data.content)
    versions = list(content['releases'].keys())

    return versions


if __name__ == "__main__":

    if sys.argv[1] == 'commitcheck':
        try:
            print('Found internal version:', internal_version())
        except ValueError as err:
            print('Repository versions are not consistent', file=sys.stderr)
            sys.exit(1)

    if sys.argv[1] == 'releasecheck':
        try:
            iver = internal_version()
            print('Found internal version:', iver)
        except ValueError as err:
            print('Repository versions are not consistent', file=sys.stderr)
            sys.exit(1)

        try:
            pypivers = pypi_versions()
            print('Found all PyPI versions:', pypivers)
        except Exception as exc:
            print('A problem occurred when checking PyPI versions', exc, \
                file=sys.stderr)
            sys.exit(2)

        if iver in pypivers:
            print('An identical release exists on PyPI; bump versions '
                'before proceeding', file=sys.stderr)
            sys.exit(1)

    sys.exit(0)
