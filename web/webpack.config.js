const webpack = require('webpack');
const ExtractTextPlugin = require("extract-text-webpack-plugin");
const HtmlWebpackPlugin = require('html-webpack-plugin');

const extractLess = new ExtractTextPlugin({
  filename: '[name].css'
});

module.exports = {
  entry: "./src/index.tsx",
  output: {
    filename: "bundle.js",
    path: __dirname + "/dist"
  },

  // Enable sourcemaps for debugging webpack's output.
  devtool: "source-map",

  resolve: {
    // Add '.ts' and '.tsx' as resolvable extensions.
    extensions: [".webpack.js", ".web.js", ".ts", ".tsx", ".js"]
  },

  plugins: [
    new webpack.DefinePlugin({
      'process.env.API_SERVER': JSON.stringify(
        process.env.API_SERVER || 'http://localhost:8000'
      ),
      'process.env.WS_SERVER': JSON.stringify(
        process.env.WS_SERVER || 'ws://localhost:8081'
      ),
    }),
    extractLess,
    new HtmlWebpackPlugin({
      title: 'brdg.me',
      hash: true,
      template: 'src/index.ejs',
    }),
  ],

  module: {
    rules: [
      // All files with a '.ts' or '.tsx' extension will be handled by 'awesome-typescript-loader'.
      { test: /\.tsx?$/, loader: "awesome-typescript-loader" },
      // All output '.js' files will have any sourcemaps re-processed by 'source-map-loader'.
      { enforce: 'pre', test: /\.js$/, loader: "source-map-loader" },
      // Less
      {
        test: /\.less$/,
        use: extractLess.extract({
          use: [{
            loader: "css-loader"
          }, {
            loader: "less-loader"
          }],
          // use style-loader in development
          fallback: "style-loader"
        })
      }
    ]
  },

  // When importing a module whose path matches one of the following, just
  // assume a corresponding global variable exists and use that instead.
  // This is important because it allows us to avoid bundling all of our
  // dependencies, which allows browsers to cache those libraries between builds.
  externals: {
    "classnames": "classNames",
    "immutable": "Immutable",
    "moment": "moment",
    "react": "React",
    "react-dom": "ReactDOM",
    "react-redux": "ReactRedux",
    "redux": "Redux",
    "redux-saga": "ReduxSaga",
    "superagent": "superagent",
  },
};
